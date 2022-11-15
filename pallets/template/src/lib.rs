#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;


#[frame_support::pallet]
pub mod pallet {
	// use frame_benchmarking::to_origin;
	use frame_system::pallet_prelude::*;
	use frame_support::pallet_prelude::*;
	// use frame_system::Event;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	// The pallet's runtime storage items:
	#[pallet::storage]
	pub type Value<T: Config> = StorageMap<_, Blake2_128Concat ,T::AccountId, u64>;

	#[pallet::storage]
	#[pallet::getter(fn get_bool)]
	pub type MyBool<T: Config> = StorageValue<_, bool>;

	// Pallets use events to inform users when important changes are made
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		SomethingStored(u32, T::AccountId),
	}

	// Errors inform users that something went wrong
	pub enum Error {
		NoneValue,
		StorageOverflow,
	}

	// Dispatchable functions allow users to interact with the pallet and invoke state changes
	#[pallet::call]
	impl<T: Config> Pallet<T> {

		#[pallet::weight(0)]
		pub fn my_function(origin: OriginFor<T>, input_bool: bool) -> DispatchResult {
			let _sender = ensure_signed(origin)?;
			<MyBool<T>>::put(input_bool);
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_value(origin: OriginFor<T>, input_value: u64) -> DispatchResult {
			let _sender = ensure_signed(origin)?;
			<Value<T>>::insert(_sender, input_value);
			Ok(())
		}
	}
}


