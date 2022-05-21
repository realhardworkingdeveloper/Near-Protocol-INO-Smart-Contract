use std::convert::TryFrom;
use std::convert::TryInto;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, NonFungibleTokenMetadataProvider, TokenMetadata, NFT_METADATA_SPEC,
};
use near_contract_standards::non_fungible_token::{Token, TokenId};
use near_contract_standards::non_fungible_token::NonFungibleToken;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{UnorderedMap, LazyOption, UnorderedSet};
use near_sdk::json_types::*;
use near_sdk::{
    env, near_bindgen, AccountId, Balance, BorshStorageKey, PanicOnDefault, Promise, PromiseOrValue, CryptoHash,
};
use near_sdk::serde::{Deserialize, Serialize};

near_sdk::setup_alloc!();

const MULTIPLYER:Balance = 10_000_000_000_000_000;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    tokens: NonFungibleToken,
    metadata: LazyOption<NFTContractMetadata>,
    total_count: u128,
    minted_count: u128,
    is_minted_by_id: UnorderedMap<u128, bool>,
    mint_price: Balance,
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    NonFungibleToken,
    Metadata,
    TokenMetadata,
    Enumeration,
    Approval,
    TokensPerOwner { account_hash: Vec<u8> },
    TokenPerOwnerInner { account_id_hash: CryptoHash },
}

#[near_bindgen]
impl Contract {

    #[init]
    pub fn new(owner_id: ValidAccountId, metadata: NFTContractMetadata, price: Balance, count: u128) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        
        metadata.assert_valid();
        
        Self {
            tokens: NonFungibleToken::new(
                StorageKey::NonFungibleToken,
                owner_id,
                Some(StorageKey::TokenMetadata),
                Some(StorageKey::Enumeration),
                Some(StorageKey::Approval),
            ),
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
            total_count: count,
            minted_count: 0,
            is_minted_by_id: UnorderedMap::new(b"is_minted_by_id".to_vec()),
            mint_price: price
        }
    }

    #[payable]
    pub fn nft_mint(
        &mut self,
        receiver_id: ValidAccountId,
    ) -> Token {

        if env::attached_deposit() < self.mint_price * MULTIPLYER {
            env::panic(b"Shoule be deposit mint price");
        }

        if self.minted_count >= self.total_count {
            env::panic(b"All nfts are minted.");
        }

        let mut token_id: u128 = 0;
        
        let mut rng: StdRng = SeedableRng::from_seed(env::random_seed().try_into().unwrap());

        let remain_count: u128 = self.total_count - self.minted_count;
        
        let random_id: u128 = rng.gen_range(0, remain_count) + 1;
        let mut passed_id: u128 = 0;

        for idx in 0..self.total_count {
            match self.is_minted_by_id.get(&idx) {
                None => {
                    passed_id += 1;

                    if passed_id == random_id {
                        token_id = idx;
                        break;
                    }
                },
                Some(_data) => {},
            }
        }

        self.is_minted_by_id.insert(&token_id, &true);
        self.minted_count += 1;

        let base_uri = match self.metadata.get().unwrap().base_uri {
            None => {
                "".to_string()
            },
            Some(data) => data,
        };

        self.tokens.custom_mint(
            token_id.to_string(), 
            // ValidAccountId::try_from(env::predecessor_account_id()).unwrap(), 
            receiver_id,
            Some(
                TokenMetadata {
                    title: Some(format!("{} #{}", self.metadata.get().unwrap().name, token_id.to_string())),
                    description: Some(format!("{}, minted by Dao Nation", self.metadata.get().unwrap().name)),
                    media: Some(format!("{}/{}.png", base_uri, token_id.to_string())), 
                    media_hash: None, 
                    copies: Some(1), 
                    issued_at: None, 
                    expires_at: None, 
                    starts_at: None, 
                    updated_at: None, 
                    extra: None, 
                    reference: Some(format!("{}/{}.json", base_uri, token_id.to_string())), 
                    reference_hash: None
                }
            ),
            self.mint_price * MULTIPLYER
        )
    }

    pub fn get_minted(&self) -> u128 {
        self.minted_count
    }
}

near_contract_standards::impl_non_fungible_token_core!(Contract, tokens);
near_contract_standards::impl_non_fungible_token_approval!(Contract, tokens);
near_contract_standards::impl_non_fungible_token_enumeration!(Contract, tokens);

#[near_bindgen]
impl NonFungibleTokenMetadataProvider for Contract {
    fn nft_metadata(&self) -> NFTContractMetadata {
        self.metadata.get().unwrap()
    }
}

pub trait Custom_NonFungibleTokenCore {
    fn custom_mint(
        &mut self, 
        token_id: TokenId, 
        token_owner_id: ValidAccountId, 
        token_metadata: Option<TokenMetadata>,
        price: Balance,
    ) -> Token;
}

impl Custom_NonFungibleTokenCore for NonFungibleToken {
    fn custom_mint(
        &mut self,
        token_id: TokenId,
        token_owner_id: ValidAccountId,
        token_metadata: Option<TokenMetadata>,
        price: Balance,
    ) -> Token {
        let initial_storage_usage = env::storage_usage();
        
        if self.token_metadata_by_id.is_some() && token_metadata.is_none() {
            env::panic(b"Must provide metadata");
        }
        if self.owner_by_id.get(&token_id).is_some() {
            env::panic(b"token_id must be unique");
        }

        let owner_id: AccountId = token_owner_id.into();

        // Core behavior: every token must have an owner
        self.owner_by_id.insert(&token_id, &owner_id);

        // Metadata extension: Save metadata, keep variable around to return later.
        // Note that check above already panicked if metadata extension in use but no metadata
        // provided to call.
        self.token_metadata_by_id
            .as_mut()
            .and_then(|by_id| by_id.insert(&token_id, &token_metadata.as_ref().unwrap()));

        // Enumeration extension: Record tokens_per_owner for use with enumeration view methods.
        if let Some(tokens_per_owner) = &mut self.tokens_per_owner {
            let mut token_ids = tokens_per_owner.get(&owner_id).unwrap_or_else(|| {
                UnorderedSet::new(StorageKey::TokensPerOwner {
                    account_hash: env::sha256(owner_id.as_bytes()),
                })
            });
            token_ids.insert(&token_id);
            tokens_per_owner.insert(&owner_id, &token_ids);
        }

        // Approval Management extension: return empty HashMap as part of Token
        let approved_account_ids =
            if self.approvals_by_id.is_some() { Some(HashMap::new()) } else { None };

        // Return any extra attached deposit not used for storage
        refund_deposit(env::storage_usage() - initial_storage_usage, price);

        Token { token_id, owner_id, metadata: token_metadata, approved_account_ids }
    }
}

pub(crate) fn refund_deposit(storage_used: u64, price: Balance) {
    //get how much it would cost to store the information
    let required_cost = env::storage_byte_cost() * Balance::from(storage_used) + price;
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