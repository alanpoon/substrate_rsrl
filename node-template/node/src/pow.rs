use sp_core::{U256, H256};
use sp_runtime::generic::BlockId;
use sp_runtime::traits::{
	Block as BlockT, Header as HeaderT, UniqueSaturatedInto,
};
use sp_api::{ProvideRuntimeApi};
use sc_client::{blockchain::HeaderBackend};
use sc_client_api::{backend::AuxStore};
use sp_runtime::codec::{Encode, Decode};
use sc_consensus_pow::{PowAlgorithm, Error};
use sp_consensus_pow::{Seal as RawSeal,Sealer,Difficulty,DifficultyApi};
use sha3::{Sha3_256, Digest};
use rand::{thread_rng, SeedableRng, rngs::SmallRng};
use std::time::Duration;
use std::sync::Arc;
use std::cell::RefCell;
use sp_std::vec::Vec;
use policy_primitives::{AlgorithmApi};
use serde_cbor;
use rsrl::{
	control::{ac::A2C, td::SARSA},
	domains::{Domain, MountainCar},
	fa::linear::{
			basis::{Fourier, Projector},
			optim::SGD,
			LFA,VectorFunction
	},
	logging,
	make_shared,
	policies::Gibbs,
	run,
	spaces::{Card,Space},
	Evaluation,
	SerialExperiment,
};

#[derive(Clone, PartialEq, Eq, Encode, Decode, Debug)]
pub struct Calculation {
	pub difficulty: Difficulty,
	pub pre_hash: H256,
	pub nonce: H256,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Compute {
	pub pre_hash: H256,
	pub difficulty: Difficulty,
	pub nonce: H256,
}
thread_local!(static MACHINES: RefCell<Option<H256>> = RefCell::new(None));

impl Compute {
	pub fn compute(self,initial_policy:Option<Vec<u8>>,rounds:usize) -> Sealer {
		println!("start compute");
		MACHINES.with(|m|{
			let domain = MountainCar::default();

			let n_actions:usize = domain.action_space().card().into();
			let bases = Fourier::from_space(3, domain.state_space()).with_constant();
			let policy = if let(Some(policy))=initial_policy{
				make_shared({
					Gibbs::standard(serde_cbor::de::from_slice(&policy).unwrap())
				})
			}else{
				make_shared({
					let fa = LFA::vector(bases.clone(), SGD(1.0), n_actions);
					Gibbs::standard(fa)
				})
			};

			let critic = {
					let q_func = LFA::vector(bases, SGD(1.0), n_actions);

					SARSA::new(q_func, policy.clone(), 0.001, 1.0)
			};

			let mut agent = A2C::new(critic, policy, 0.001);

			let logger = logging::root(logging::stdout());
			let domain_builder = Box::new(MountainCar::default);
			
			// Training phase:
			let _training_result = {
					let e = SerialExperiment::new(&mut agent, domain_builder.clone(), 1000);
					run(e, rounds, Some(logger.clone()))
			};
			let policy = serde_cbor::ser::to_vec(&agent.policy().fa()).unwrap();
			// Testing phase:
			let testing_result = Evaluation::new(&mut agent, domain_builder).next().unwrap();
			info!(logger, "solution"; testing_result.clone());
			let calculation = Calculation {
				difficulty: self.difficulty,
				pre_hash: self.pre_hash,
				nonce: self.nonce,
			};
			println!("difficulty {:?}",self.difficulty);
			Sealer {
				nonce: self.nonce,
				difficulty: self.difficulty,
				policy: policy,
				steps:testing_result.steps
			}
		})
	}
}
//#[derive(Clone)]
pub struct Sha3Algorithm<C> {
	client: Arc<C>,
}
impl<C> Sha3Algorithm<C> {
	pub fn new(client: Arc<C>) -> Self {
		Self { client }
	}
}

impl<C> Clone for Sha3Algorithm<C> {
	fn clone(&self) -> Self {
		Self { client: self.client.clone() }
	}
}

impl<B: BlockT<Hash=H256>, C> PowAlgorithm<B> for Sha3Algorithm<C>where
C: HeaderBackend<B> + AuxStore + ProvideRuntimeApi<B>,
C::Api: DifficultyApi<B, Difficulty> + AlgorithmApi<B>, {
	type Difficulty = Difficulty;

	fn difficulty(&self, parent: &BlockId<B>) -> Result<Difficulty, Error<B>> {
		let difficulty = self.client.runtime_api().difficulty(parent)
			.map_err(|e| sc_consensus_pow::Error::Environment(
				format!("Fetching difficulty from runtime failed: {:?}", e)
			));

		difficulty
	}
	fn policy(&self, parent: &BlockId<B>) -> Result<Option<Vec<u8>>, sc_consensus_pow::Error<B>> {
		let policy = self.client.runtime_api().policy(parent)
			.map_err(|e| sc_consensus_pow::Error::Environment(
				format!("Fetching policy from runtime failed: {:?}", e)
			));
		policy
	}

	fn verify(
		&self,
		parent: &BlockId<B>,
		pre_hash: &H256,
		seal: &RawSeal,
		difficulty: Difficulty,
		policy: Option<Vec<u8>>,
	) -> Result<bool, Error<B>> {
		let seal = match Sealer::decode(&mut &seal[..]) {
			Ok(seal) => seal,
			Err(_) => return Ok(false),
		};
		let old_steps = seal.steps;
		let compute = Compute {
			difficulty,
			pre_hash: *pre_hash,
			nonce: seal.nonce
		};

		if compute.compute(policy,1).steps >= old_steps {
			return Ok(false)
		}
		Ok(true)
	}

	fn mine(
		&self,
		parent: &BlockId<B>,
		pre_hash: &H256,
		difficulty: Difficulty,
		policy: Option<Vec<u8>>,
	) -> Result<Option<RawSeal>, Error<B>> {
		let mut rng = SmallRng::from_rng(&mut thread_rng())
			.map_err(|e| Error::Environment(format!("Initialize RNG failed for mining: {:?}", e)))?;
		let nonce = H256::random_using(&mut rng);
		let compute = Compute {
			difficulty,
			pre_hash: *pre_hash,
			nonce,
		};
		println!("diff {:?} initial_policy {:?}",difficulty.as_usize(), policy.clone());
		let seal = compute.compute(policy,difficulty.as_usize());
		Ok(Some(seal.encode()))
	}
}
