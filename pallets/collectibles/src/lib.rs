#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use frame_support::{traits::tokens::currency::Currency, traits::Randomness};
	use frame_support::sp_runtime::app_crypto::sp_core::blake2_128;

	type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[derive(Clone, Encode, Decode, PartialEq, Copy, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub enum Color {
		Red,
		Yellow,
		Blue,
		Green,
	}

	#[derive(Clone, Encode, Decode, PartialEq, Copy, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	#[codec(mel_bound())]
	pub struct Collectible<T: Config> {
		pub unique_id: [u8; 16],
		pub price: Option<BalanceOf<T>>,
		pub color: Color,
		pub owner: T::AccountId,
	}

	// impl Default for Collectible {
	// 	fn default() -> Self {
	// 		Collectible {
	// 			unique_id: Self::generate_id().0,
	// 			price: None,
	// 			color: Color::Red,
	// 			owner: T::,
	// 		}
	// 	}
	// }

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	// Declare storage items for the pallet
	#[pallet::storage]
	pub(super) type CollectiblesCount<T: Config> = StorageValue<_, u64, ValueQuery>;
	/// Maps the collectibles struct to the unique ID:
	#[pallet::storage]
	pub(super) type CollectiblesMap<T: Config> = StorageMap<_, Twox64Concat, [u8; 16], Collectible<T>, ValueQuery>;
	/// Track the collectibles owned by each account
	#[pallet::storage]
	pub(super) type OwnerOfCollectibles<T: Config> = StorageMap<_, Twox64Concat,
		T::AccountId, BoundedVec<[u8; 16], T::MaxOwned>,
		ValueQuery
	>;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type Currency: Currency<Self::AccountId>;

		type Randomness: Randomness<Self::Hash, Self::BlockNumber>;

		#[pallet::constant]
		type MaxOwned: Get<u32>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Each collectible must have a unique identifier
		DuplicateCollectible,
		/// An account can't exceed the MaxOwned constant
		MaximumCollectiblesOwned,
		/// The total supply of collectibles can't exceed the u64 limit
		BoundsOverflow
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new collectible created
		CollectibleCreated {
			collectible: [u8; 16],
			owner: T::AccountId
		}
	}

	// pallet callable functions
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn create_collectible(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// Generate the unique_id and color
			let (collectible_id, color) = Self::generate_id();

			// Write new collectible to storage
			Self::mint(&sender, collectible_id, color)?;
			Ok(())
		}
	}

	// pallet internal functions
	impl<T: Config> Pallet<T> {

		fn generate_id() -> ([u8; 16], Color) {
			let payload = (
				T::Randomness::random(&b"unique_id"[..]).0,
				frame_system::Pallet::<T>::extrinsic_index().unwrap_or_default(),
				frame_system::Pallet::<T>::block_number(),
			);

			// Turns into byte array
			let encoded_payload = payload.encode();
			let hash = frame_support::Hashable::blake2_128(&encoded_payload);

			// Generate Color
			if hash[0] % 2 == 0 {
				return (hash, Color::Red);
			}
			(hash, Color::Yellow)
		}

		pub fn mint(
			owner: &T::AccountId,
			unique_id: [u8; 16],
			color: Color,
		) -> Result<[u8; 16], DispatchError> {
			let collectible = Collectible::<T> {
				unique_id,
				price: None,
				color,
				owner: owner.clone()
			};

			// Check if the collectible exists in the storage map
			ensure!(!CollectiblesMap::<T>::contains_key(&collectible.unique_id), Error::<T>::DuplicateCollectible);

			// Check that a new collectible can be created
			let count = CollectiblesCount::<T>::get();
			let new_count = count.checked_add(1).ok_or(Error::<T>::BoundsOverflow);

			// Append collectible to the OwnerOfCollectibles map
			OwnerOfCollectibles::<T>::try_append(&owner, collectible.unique_id)
				.map_err(|_| Error::<T>::MaximumCollectiblesOwned)?;

			// Write new collectible to storage and update the count
			CollectiblesMap::<T>::insert(collectible.unique_id, collectible);
			CollectiblesCount::<T>::put(new_count);

			// Deposit the CollectibleCreated event
			Self::deposit_event(Event::CollectibleCreated {
				collectible: unique_id, owner: owner.clone()
			});

			// Returns the unique_id of the new collectible
			Ok(unique_id)
		}
	}
}
