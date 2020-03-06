//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use std::sync::Arc;
use std::time::Duration;
use sc_client::LongestChain;
use node_template_runtime::{self, GenesisConfig, opaque::Block, RuntimeApi};
use sc_service::{error::{Error as ServiceError}, AbstractService, Configuration, ServiceBuilder};
use sp_inherents::InherentDataProviders;
use sc_network::{construct_simple_protocol,config::DummyFinalityProofRequestBuilder};
use sc_executor::native_executor_instance;
pub use sc_executor::NativeExecutor;
use sp_consensus_aura::sr25519::{AuthorityPair as AuraPair};
use grandpa::{self, FinalityProofProvider as GrandpaFinalityProofProvider};
use crate::pow::{Sha3Algorithm};
// Our native executor instance.
native_executor_instance!(
	pub Executor,
	node_template_runtime::api::dispatch,
	node_template_runtime::native_version,
);

construct_simple_protocol! {
	/// Demo protocol attachment for substrate.
	pub struct NodeProtocol where Block = Block { }
}

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
macro_rules! new_full_start {
	($config:expr) => {{
		let inherent_data_providers = sp_inherents::InherentDataProviders::new();

		let builder = sc_service::ServiceBuilder::new_full::<
			node_template_runtime::opaque::Block, node_template_runtime::RuntimeApi, crate::service::Executor
		>($config)?
			.with_select_chain(|_config, backend| {
				Ok(sc_client::LongestChain::new(backend.clone()))
			})?
			.with_transaction_pool(|config, client, _fetcher| {
				let pool_api = sc_transaction_pool::FullChainApi::new(client.clone());
				Ok(sc_transaction_pool::BasicPool::new(config, std::sync::Arc::new(pool_api)))
			})?
			.with_import_queue(|_config, client, mut select_chain, transaction_pool| {
				let algorithm = crate::pow::Sha3Algorithm::new(client.clone());
				let import_queue = sc_consensus_pow::import_queue(
					Box::new(client.clone()),
					client.clone(),
					algorithm,
					0,
					select_chain,
					inherent_data_providers.clone(),
				)?;
				Ok(import_queue)
			})?;

		(builder, inherent_data_providers)
	}}
}

/// Builds a new service for a full client.
pub fn new_full(config: Configuration<GenesisConfig>)
	-> Result<impl AbstractService, ServiceError>
{
	let is_authority = config.roles.is_authority();
	let name = config.name.clone();
	let disable_grandpa = config.disable_grandpa;

	// sentry nodes announce themselves as authorities to the network
	// and should run the same protocols authorities do, but it should
	// never actively participate in any consensus process.
	let participates_in_consensus = is_authority && !config.sentry_mode;
	println!("before full start");
	let (builder, inherent_data_providers) = new_full_start!(config);
	println!("after full start");
	let service = builder
		.with_network_protocol(|_| Ok(NodeProtocol::new()))?
		.with_finality_proof_provider(|_client, _backend| {
			Ok(Arc::new(()) as _)
		})?
		.build()?;

	if participates_in_consensus {
		let proposer = sc_basic_authorship::ProposerFactory {
			client: service.client(),
			transaction_pool: service.transaction_pool(),
		};

		let client = service.client();
		let round = 10;
		sc_consensus_pow::start_mine(
				Box::new(service.client().clone()),
				service.client(),
				Sha3Algorithm::new(client),
				proposer,
				None,
				round,
				service.network(),
				std::time::Duration::new(2, 0),
				service.select_chain().map(|v| v.clone()),
				inherent_data_providers.clone(),
				sp_consensus::AlwaysCanAuthor,
			);

	}

	Ok(service)
}

/// Builds a new service for a light client.
pub fn new_light(config: Configuration<GenesisConfig>)
	-> Result<impl AbstractService, ServiceError>
{
	let inherent_data_providers = InherentDataProviders::new();

	ServiceBuilder::new_light::<Block, RuntimeApi, Executor>(config)?
		.with_select_chain(|_config, backend| {
			Ok(LongestChain::new(backend.clone()))
		})?
		.with_transaction_pool(|config, client, fetcher| {
			let fetcher = fetcher
				.ok_or_else(|| "Trying to start light transaction pool without active fetcher")?;

			let pool_api = sc_transaction_pool::LightChainApi::new(client.clone(), fetcher.clone());
			let pool = sc_transaction_pool::BasicPool::with_revalidation_type(
				config, Arc::new(pool_api), sc_transaction_pool::RevalidationType::Light,
			);
			Ok(pool)
		})?
		.with_import_queue_and_fprb(|_config, client, backend, fetcher, select_chain, _tx_pool| {			
			let fprb = Box::new(DummyFinalityProofRequestBuilder::default()) as Box<_>;
			let algorithm = crate::pow::Sha3Algorithm::new(client.clone());
			let import_queue = sc_consensus_pow::import_queue(
				Box::new(client.clone()),
				client.clone(),
				algorithm,
				0,
				select_chain,
				inherent_data_providers.clone(),
			)?;

			Ok((import_queue, fprb))
		})?
		.with_network_protocol(|_| Ok(NodeProtocol::new()))?
		.with_finality_proof_provider(|client, backend|
			Ok(Arc::new(()) as _)
		)?
		.build()
}
