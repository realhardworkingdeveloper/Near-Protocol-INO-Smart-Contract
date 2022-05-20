use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::collections::*;
use near_sdk::{
    env, near_bindgen, PanicOnDefault, Balance, Promise, AccountId
};

#[derive(Debug, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Collection {
    name: String,
    symbol: String,
    url: String,
    total_count: u128,
    price: Balance,
    contract: String
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct INO {
    owner: AccountId,
    collection_count: u128,
    collection_by_id: LookupMap<u128, Collection>,
    status_by_id: UnorderedMap<u128, bool>,
}

#[near_bindgen]
impl INO {
    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "Already initialized");

        Self {
            owner: env::predecessor_account_id(),
            collection_count: 0,
            collection_by_id: LookupMap::new(b"collection_by_id".to_vec()), 
            status_by_id: UnorderedMap::new(b"status_by_id".to_vec()), 
        }
    }

    #[payable]
    pub fn add_collection(
        &mut self,
        new_collection: Collection
    ) {
        let initial_storage_usage = env::storage_usage();

        let new_id: u128 = self.collection_count;

        self.collection_by_id.insert(&new_id, &new_collection);
        self.status_by_id.insert(&new_id, &false);

        self.collection_count += 1;

        let required_storage_in_bytes = env::storage_usage() - initial_storage_usage;

        refund_deposit(required_storage_in_bytes);
    }

    pub fn update_collection_status(
        &mut self,
        arg_collection_id: Option<u128>
    ) {
        if env::predecessor_account_id() != self.owner {
            env::panic(b"Only owner could update");
        }

        let collection_id: u128 = match arg_collection_id {
            None => self.collection_count - 1,
            _ => arg_collection_id.unwrap()
        };

        if collection_id < 0 || collection_id >= self.collection_count {
            env::panic(b"Invalid collection id");
        }

        self.status_by_id.insert(&collection_id, &true);
    }

    pub fn get_collection(&self) -> (Vec<Collection>, Vec<bool>) {
        let mut collections = Vec::new();
        let mut status = Vec::new();

        let count = self.collection_count;

        for id in 0..count {
            match self.collection_by_id.get(&id).unwrap() {
                data => collections.push(data),
                _ => {},
            };

            match self.status_by_id.get(&id) {
                Some(data) => status.push(data),
                _ => {},
            };
        }

        (collections, status)
    }
}

pub(crate) fn refund_deposit(storage_used: u64) {
    //get how much it would cost to store the information
    let required_cost = env::storage_byte_cost() * Balance::from(storage_used);
    //get the attached deposit
    let attached_deposit = env::attached_deposit();

    //make sure that the attached deposit is greater than or equal to the required cost
    assert!(
        required_cost <= attached_deposit,
        "Must attach {} yoctoNEAR to cover storage",
        required_cost,
    );

    //get the refund amount from the attached deposit - required cost
    let refund = attached_deposit - required_cost;

    //if the refund is greater than 1 yocto NEAR, we refund the predecessor that amount
    if refund > 1 {
        Promise::new(env::predecessor_account_id()).transfer(refund);
    }
}