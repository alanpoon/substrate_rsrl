use frame_support::{decl_storage, decl_module,decl_event,Parameter};
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
    pub Items: map hasher(blake2_256) u32 => Option<Item>;
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