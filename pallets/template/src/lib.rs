#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
// use codec::{Encode, Decode};

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
	use sp_runtime::traits::{Hash};

	// #[cfg(feature = "std")]
	// use serde::{Serialize, Deserialize};

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
	pub type KittiesOwned<T: Config> = StorageMap<_, Twox64Concat, T::AccountId,
		BoundedVec<T::Hash, T::MaxKittiesOwned>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn owned_kitties_count)]
	pub type OwnedKittiesCount<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, u64, ValueQuery>;

	#[pallet::storage]
	pub type OwnedKittiesIndex<T: Config> = StorageMap<_, Blake2_128Concat, T::Hash, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn all_kitties_count)]
	pub type AllKittiesCount<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn kitty_by_index)]
	pub type AllKittiesArray<T: Config> = StorageMap<
		_, Blake2_128Concat,
		u64, T::Hash>;

	#[pallet::storage]
	pub type AllKittiesIndex<T: Config> = StorageMap<_, Blake2_128Concat, T::Hash, u64>;

	#[pallet::storage]
	pub type KittyData<T: Config> = StorageMap<_, Blake2_128Concat, T::Hash, Kitty<T>>;

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
		Created(T::AccountId, T::Hash),
	}

	// Errors inform users that something went wrong
	#[derive(Debug)]
	pub enum Error {
		NoKittyOwner,
		ExceedMaxKittyOwned,
		MaxKittiesOwned,
		KittyCountOverflow,
	}

	// Dispatchable functions allow users to interact with the pallet and invoke state changes
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn create_kitty(origin: OriginFor<T>) -> DispatchResult {
			let _sender = ensure_signed(origin)?;
			let kitty_id = Self::mint(&_sender, None, None);
			Self::deposit_event(Event::Created(_sender.clone(), kitty_id.unwrap()));

			Ok(())
		}
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

			let owned_kitties_count = Self::owned_kitties_count(&owner).checked_add(1).ok_or(<Error>::KittyCountOverflow)?;
			let all_kitties_count = Self::all_kitties_count().checked_add(1).ok_or(<Error>::KittyCountOverflow)?;

			let kitty_id = T::Hashing::hash_of(&kitty);

			<KittyData<T>>::insert(kitty_id, kitty);
			<KittiesOwned<T>>::try_mutate(&owner, |kitty_vec| kitty_vec.try_push(kitty_id))
				.map_err(|_| <Error>::ExceedMaxKittyOwned)?;
			<KittyOwner<T>>::insert(kitty_id, &owner);
			<AllKittiesCount<T>>::put(all_kitties_count);
			<AllKittiesArray<T>>::insert(all_kitties_count - 1, kitty_id);
			<AllKittiesIndex<T>>::insert(kitty_id, all_kitties_count - 1);
			<OwnedKittiesCount<T>>::insert(&owner, owned_kitties_count);
			<OwnedKittiesIndex<T>>::insert(kitty_id, owned_kitties_count - 1);

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


