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
use near_sdk::json_types::*;
use near_sdk::{
    env, near_bindgen, AccountId, Balance, BorshStorageKey, PanicOnDefault, Promise, PromiseOrValue,
};
use near_sdk::serde::{Deserialize, Serialize};

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
    // #[init]
    // pub fn new_default_meta(owner_id: ValidAccountId) -> Self {
    //     Self::new(
    //         owner_id,
    //         NFTContractMetadata {
    //             spec: NFT_METADATA_SPEC.to_string(),
    //             name: "Example NEAR non-fungible token".to_string(),
    //             symbol: "EXAMPLE".to_string(),
    //             icon: Some(DATA_IMAGE_SVG_NEAR_ICON.to_string()),
    //             base_uri: Some("https://gateway.pinata.cloud/ipfs/QmeRGXZH4drhsGYiZmQS5nQbBxMmuugYiC2HBns3ChpMCC".to_string()),
    //             reference: None,
    //             reference_hash: None,
    //         },
    //         6:U128,
    //         1_000_000_000_000_000_000_000_000
    //     )
    // }

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

        if env::attached_deposit() < self.mint_price * 10_000_000_000_000_000 {
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