#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
use codec::{Encode, Decode};

#[frame_support::pallet]
pub mod pallet {
	// use frame_support::pallet;
	use frame_system::pallet_prelude::*;
	use frame_support::{
		pallet_prelude::*,
		traits::{Currency},
	};
	use frame_support::traits::Randomness;
	use sp_io::hashing::blake2_128;
	// use frame_support::traits::tokens::Balance;
	use sp_runtime::traits::{AtLeast32BitUnsigned, Hash};

	#[cfg(feature = "std")]
	use serde::{Serialize, Deserialize};

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub enum Gender {
		Male,
		Female,
	}

	impl Default for Gender {
		fn default() -> Self {
			Gender::Male
		}
	}

	#[derive(Encode, Decode, Clone, PartialEq, TypeInfo, MaxEncodedLen, RuntimeDebug)]
	#[scale_info(skip_type_params(T))]
	#[codec(mel_bound())]
	pub struct Kitty<T: Config> {
		pub owner: AccountOf<T>,
		pub dna: [u8; 16],
		pub price: Option<BalanceOf<T>>,
		pub gen: Gender,
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Currency: Currency<Self::AccountId>;

		type KittyRandomness: Randomness<Self::Hash, Self::BlockNumber>;

		#[pallet::constant]
		type MaxKittiesOwned: Get<u32>;
	}

	// The pallet's runtime storage items:
	#[pallet::storage]
	#[pallet::getter(fn kitty_of_owner)]
	pub type OwnedKitty<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, T::Hash>;
	// pub type OwnedKitty<T: Config> = StorageValue<_, T::Hash>;

	#[pallet::storage]
	pub type KittiesOwned<T: Config> = StorageMap<_, Twox64Concat, T::AccountId,
		BoundedVec<T::Hash, T::MaxKittiesOwned>, ValueQuery>;


	#[pallet::storage]
	pub type Kitties<T: Config> = StorageMap<_, Blake2_128Concat, T::Hash, Kitty<T>>;

	#[pallet::storage]
	#[pallet::getter(fn owner_of)]
	pub type KittyOwner<T: Config> = StorageMap<_, Blake2_128Concat, T::Hash, T::AccountId>;

	// #[pallet::storage]
	// #[pallet::getter(fn get_bool)]
	// pub type MyBool<T: Config> = StorageValue<_, bool>;

	// Pallets use events to inform users when important changes are made
	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		SomethingStored(u32, T::AccountId),
	}

	// Errors inform users that something went wrong
	pub enum Error {
		NoKittyOwner,
		ExceedMaxKittyOwned,
		MaxKittiesOwned,
	}

	// Dispatchable functions allow users to interact with the pallet and invoke state changes
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn create_kitty(origin: OriginFor<T>) -> DispatchResult {
			let _sender = ensure_signed(origin)?;
			let kitty_id = Self::mint(&_sender, None, None);
			// let nonce = <Nonce<T>>::get();
			// let random_seed = <frame_system::Pallet<T>>::random_seed();
			//
			// let random_hash = (random_seed, _sender, nonce).using_encoded(<T as frame_system::Config>::Hashing::hash(&[0]));
			// ensure!(!<KittyOwner<T>>::exists(random_hash), "Kitty already exists");

			// let new_kitty = Kitty::<T> {
			// 	owner: _sender.clone(),
			// 	dna: ,
			// 	price: 0,
			// 	gen: 0,
			// };
			//
			// <Kitties<T>>::insert(random_hash, new_kitty);
			// <KittyOwner<T>>::insert(random_hash, &_sender);
			// <OwnedKitty<T>>::insert(&_sender, random_hash);
			// <Nonce<T>>::mutate(|n| *n += 1);
			Ok(())
		}

		// pub fn my_function(origin: OriginFor<T>, input_bool: bool) -> DispatchResult {
		// 	let _sender = ensure_signed(origin)?;
		// 	<MyBool<T>>::put(input_bool);
		// 	Ok(())
		// }
		//
		// #[pallet::weight(0)]
		// pub fn set_value(origin: OriginFor<T>, input_value: u64) -> DispatchResult {
		// 	let _sender = ensure_signed(origin)?;
		// 	<Value<T>>::insert(_sender, input_value);
		// 	Ok(())
		// }
	}

	impl<T: Config> Kitty<T> {
		pub fn gender(dna: T::Hash) -> Gender {
			if dna.as_ref()[0] % 2 == 0 {
				Gender::Male
			} else {
				Gender::Female
			}
		}
	}

	impl<T: Config> Pallet<T> {
		fn mint(
			owner: &T::AccountId,
			gender: Option<Gender>,
			dna: Option<[u8; 16]>,
		) -> Result<T::Hash, Error> {
			let kitty = Kitty::<T> {
				dna: dna.unwrap_or_else(Self::generate_dna),
				price: None,
				gen: gender.unwrap_or_else(Self::generate_gender),
				owner: owner.clone(),
			};

			let kitty_id = T::Hashing::hash_of(&kitty);

			<Kitties<T>>::insert(kitty_id, kitty);
			<KittiesOwned<T>>::try_mutate(&owner, |kitty_vec| kitty_vec.try_push(kitty_id))
				.map_err(|_| <Error>::ExceedMaxKittyOwned)?;
			<OwnedKitty<T>>::insert(&owner, kitty_id);
			<KittyOwner<T>>::insert(kitty_id, &owner);

			Ok(kitty_id)
		}

		fn generate_dna() -> [u8; 16] {
			let payload = (
				T::KittyRandomness::random(&b"gender"[..]).0,
				<frame_system::Pallet<T>>::block_number(),
			);

			payload.using_encoded(blake2_128)
		}

		fn generate_gender() -> Gender {
			let random = T::KittyRandomness::random(&b"gender"[..]).0;
			match random.as_ref()[0] % 2 {
				0 => Gender::Male,
				_ => Gender::Female
			}
		}
	}
}


