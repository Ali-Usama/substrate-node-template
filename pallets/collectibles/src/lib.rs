#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use frame_support::{traits::tokens::currency::Currency, traits::Randomness};
	use frame_support::sp_runtime::app_crypto::sp_core::blake2_128;
	use frame_support::sp_runtime::traits::IntegerSquareRoot;

	type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[derive(Encode, Decode, Clone, PartialEq, TypeInfo, MaxEncodedLen, RuntimeDebug)]
	pub enum Color {
		Red,
		Yellow,
		Blue,
		Green,
	}

	impl Default for Color {
		fn default() -> Self {
			Color::Red
		}
	}

	#[derive(Encode, Decode, Clone, PartialEq, TypeInfo, MaxEncodedLen, RuntimeDebug)]
	#[scale_info(skip_type_params(T))]
	#[codec(mel_bound())]
	pub struct Collectible<T: Config> {
		pub unique_id: [u8; 16],
		pub price: Option<BalanceOf<T>>,
		pub color: Color,
		pub owner: T::AccountId,
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	// Declare storage items for the pallet
	#[pallet::storage]
	pub(super) type CollectiblesCount<T: Config> = StorageValue<_, u64, ValueQuery>;
	/// Maps the collectibles struct to the unique ID:
	#[pallet::storage]
	pub(super) type CollectiblesMap<T: Config> = StorageMap<_, Twox64Concat, [u8; 16], Collectible<T>>;
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
		BoundsOverflow,
		/// The collectible doesn't exist
		NoCollectible,
		/// You are not the owner
		NotOwner,
		/// Trying to transfer a collectible to yourself
		TransferToYourself,
		/// The bid is lower than the ask price
		BidPriceTooSlow,
		/// The collectible is nor for sale
		NotForSale,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new collectible created
		CollectibleCreated {
			collectible: [u8; 16],
			owner: T::AccountId
		},
		/// A collectible was successfully transferred
		TransferSucceeded {
			collectible: [u8; 16],
			from: T::AccountId,
			to: T::AccountId,
		},
		/// Price was updated successfully
		PriceSet {
			collectible: [u8; 16],
			price: Option<BalanceOf<T>>,
		},
		/// Collectible was sold successfully
		CollectibleSold {
			collectible: [u8; 16],
			seller: T::AccountId,
			buyer: T::AccountId,
			price: Option<BalanceOf<T>>,
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

		/// Transfer a collectible to another account
		/// Transfer resets the price of the collectible, making it not for Sale
		#[pallet::weight(0)]
		pub fn transfer(
			origin: OriginFor<T>,
			to: T::AccountId,
			unique_id: [u8; 16],
		) -> DispatchResult {
			let from = ensure_signed(origin)?;
			let collectible = CollectiblesMap::<T>::get(&unique_id)
				.ok_or(Error::<T>::NoCollectible)?;
			ensure!(collectible.owner == from, Error::<T>::NotOwner);
			Self::do_transfer(unique_id, to)?;
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_price(
			origin: OriginFor<T>,
			price: Option<BalanceOf<T>>,
			unique_id: [u8; 16],
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// Check if the collectible exists
			let mut  collectible = CollectiblesMap::<T>::get(&unique_id)
				.ok_or(Error::<T>::NoCollectible)?;
			ensure!(collectible.owner == sender, Error::<T>::NotOwner );

			// set the price in storage
			collectible.price = price;
			CollectiblesMap::<T>::insert(&unique_id, collectible);

			// Deposit the 'PriceSet' event
			Self::deposit_event(Event::PriceSet {
				collectible: unique_id,
				price,
			});

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn buy_collective(
			origin: OriginFor<T>,
			unique_id: [u8; 16],
			bid_price: BalanceOf<T>,
		) -> DispatchResult {
			let buyer = ensure_signed(origin)?;
			Self::do_buy_collectible(bid_price, unique_id, buyer)?;
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
			let new_count = count.checked_add(1).ok_or(Error::<T>::BoundsOverflow)?;

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

		pub fn do_transfer(
			unique_id: [u8; 16],
			to: T::AccountId
		) -> DispatchResult {
			// Get the collectible
			let mut collectible = CollectiblesMap::<T>::get(&unique_id)
				.ok_or(Error::<T>::NoCollectible)?;
			let from = collectible.owner;

			ensure!(from != to, Error::<T>::TransferToYourself);
			let mut from_owned = OwnerOfCollectibles::<T>::get(&from);

			// Remove collectible from list of owned collectible
			if let Some(ind) = from_owned.iter().position(|&id| id == unique_id) {
				from_owned.swap_remove(ind);
			} else {
				return Err(Error::<T>::NoCollectible.into())
			}

			// Add collectible to the list of owned collectibles
			let mut to_owned = OwnerOfCollectibles::<T>::get(&to);
			to_owned.try_push(unique_id).map_err(|()| Error::<T>::MaximumCollectiblesOwned)?;

			// Transfer succeeded, update the owner and reset the price to 'None'
			collectible.owner = to.clone();
			collectible.price = None;

			// Write updates to storage
			CollectiblesMap::<T>::insert(&unique_id, collectible);
			OwnerOfCollectibles::<T>::insert(&to, to_owned);
			OwnerOfCollectibles::<T>::insert(&from, from_owned);

			Ok(())
		}

		pub fn do_buy_collectible(
			bid_price: BalanceOf<T>,
			unique_id: [u8; 16],
			buyer: T::AccountId,
		) -> DispatchResult {
			// Get the collectible from the storage
			let mut collectible = CollectiblesMap::<T>::get(&unique_id)
				.ok_or(Error::<T>::NoCollectible)?;
			let seller = collectible.owner;

			ensure!(seller != buyer, Error::<T>::TransferToYourself);
			let mut from_owned = OwnerOfCollectibles::<T>::get(&seller);

			// Remove collectible from owner collectibles
			if let Some(ind) = from_owned.iter().position(|&id| id == unique_id) {
				from_owned.swap_remove(ind);
			} else {
				return Err(Error::<T>::NoCollectible.into())
			};

			// Add collectible to the buyer collectibles
			let mut buyer_owned = OwnerOfCollectibles::<T>::get(&buyer);
			buyer_owned.try_push(unique_id).map_err(|()| Error::<T>::MaximumCollectiblesOwned)?;

			// Mutating the state with a balance transfer, so nothing is allowed to fail after tihs
			if let Some(price) = collectible.price {
				ensure!(bid_price >= price, Error::<T>::BidPriceTooSlow);
				// Transfer the amount from buyer to seller
				T::Currency::transfer(
					&seller,
					&buyer,
					price,
					frame_support::traits::ExistenceRequirement::KeepAlive
				)?;

				// Deposit the Sold event
				Self::deposit_event(Event::<T>::CollectibleSold {
					seller: seller.clone(),
					buyer: buyer.clone(),
					price: Some(price),
					collectible: unique_id
				});
			} else {
				return  Err(Error::<T>::NotForSale.into())
			}

			// Transfer succeeded, update the collectible owner and reset the price
			collectible.owner = buyer.clone();
			collectible.price = None;

			// Write updates to storage
			CollectiblesMap::<T>::insert(&unique_id, collectible);
			OwnerOfCollectibles::<T>::insert(&buyer, buyer_owned);
			OwnerOfCollectibles::<T>::insert(&seller, from_owned);
			Self::deposit_event(Event::<T>::TransferSucceeded {
				collectible: unique_id,
				from: seller,
				to: buyer
			});
			Ok(())
		}
	}
}
