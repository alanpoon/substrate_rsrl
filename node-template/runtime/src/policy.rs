use frame_support::{decl_storage, decl_module,decl_event,Parameter,StorageMap};
use sp_runtime::codec::{Encode, Decode};
use sp_std::vec::Vec;
pub trait Trait: system::Trait{
  type Item: Parameter;
  type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}
#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Item {
  policy:Vec<u8>
}
decl_storage! {
	trait Store for Module<T: Trait> as Policy {
		//pub Items: map hasher(blake2_256) u32 => Option<Item>;
		pub Now get(fn now) build(|_| vec![]): Vec<u8>;
	}
}
impl<T: Trait> Module<T> {
	/// Get the current time for the current block.
	///
	/// NOTE: if this function is called prior to setting the timestamp,
	/// it will return the timestamp of the previous block.
	pub fn get() -> Vec<u8> {
		Self::now()
	}
	pub fn set_policy(now: Vec<u8>) {
		<Self as Store>::Now::put(now);
	}
}
decl_event! {
	/// Events type.
	pub enum Event<T> where <T as system::Trait>::AccountId,
	{
    ItemCreated(AccountId),
	}
}
decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn on_finalize(_n: T::BlockNumber) {

    }
  }
}
