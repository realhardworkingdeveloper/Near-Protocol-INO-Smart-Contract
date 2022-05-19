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
use near_sdk::collections::{UnorderedMap, LazyOption};
use near_sdk::json_types::ValidAccountId;
use near_sdk::{
    env, near_bindgen, AccountId, Balance, BorshStorageKey, PanicOnDefault, Promise, PromiseOrValue,
};

near_sdk::setup_alloc!();

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

const DATA_IMAGE_SVG_NEAR_ICON: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 288 288'%3E%3Cg id='l' data-name='l'%3E%3Cpath d='M187.58,79.81l-30.1,44.69a3.2,3.2,0,0,0,4.75,4.2L191.86,103a1.2,1.2,0,0,1,2,.91v80.46a1.2,1.2,0,0,1-2.12.77L102.18,77.93A15.35,15.35,0,0,0,90.47,72.5H87.34A15.34,15.34,0,0,0,72,87.84V201.16A15.34,15.34,0,0,0,87.34,216.5h0a15.35,15.35,0,0,0,13.08-7.31l30.1-44.69a3.2,3.2,0,0,0-4.75-4.2L96.14,186a1.2,1.2,0,0,1-2-.91V104.61a1.2,1.2,0,0,1,2.12-.77l89.55,107.23a15.35,15.35,0,0,0,11.71,5.43h3.13A15.34,15.34,0,0,0,216,201.16V87.84A15.34,15.34,0,0,0,200.66,72.5h0A15.35,15.35,0,0,0,187.58,79.81Z'/%3E%3C/g%3E%3C/svg%3E";

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    NonFungibleToken,
    Metadata,
    TokenMetadata,
    Enumeration,
    Approval,
}

#[near_bindgen]
impl Contract {
    /// Initializes the contract owned by `owner_id` with
    /// default metadata (for example purposes only).
    #[init]
    pub fn new_default_meta(owner_id: ValidAccountId) -> Self {
        Self::new(
            owner_id,
            NFTContractMetadata {
                spec: NFT_METADATA_SPEC.to_string(),
                name: "Example NEAR non-fungible token".to_string(),
                symbol: "EXAMPLE".to_string(),
                icon: Some(DATA_IMAGE_SVG_NEAR_ICON.to_string()),
                base_uri: Some("https://gateway.pinata.cloud/ipfs/QmeRGXZH4drhsGYiZmQS5nQbBxMmuugYiC2HBns3ChpMCC".to_string()),
                reference: None,
                reference_hash: None,
            },
            6,
            1_000_000_000_000_000_000_000_000
        )
    }

    #[init]
    pub fn new(owner_id: ValidAccountId, metadata: NFTContractMetadata, count: u128, price: Balance) -> Self {
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

        if env::attached_deposit() != self.mint_price {
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

        for idx in 0..remain_count {
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

        self.tokens.mint(
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
            )
        )
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