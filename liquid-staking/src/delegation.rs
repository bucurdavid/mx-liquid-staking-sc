use crate::errors::{
    ERROR_ALREADY_WHITELISTED, ERROR_CLAIM_EPOCH, ERROR_CLAIM_REDELEGATE, ERROR_CLAIM_START,
    ERROR_NOT_WHITELISTED,
};

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, PartialEq, Eq, TypeAbi, Clone)]
pub enum ClaimStatusType {
    None,
    Pending,
    Finished,
    Redelegated,
}

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, PartialEq, Eq, TypeAbi, Clone)]
pub struct ClaimStatus<M: ManagedTypeApi> {
    pub status: ClaimStatusType,
    pub last_claim_epoch: u64,
    pub current_iteration: usize,
    pub starting_token_reserve: BigUint<M>,
}

impl<M: ManagedTypeApi + CryptoApi> Default for ClaimStatus<M> {
    fn default() -> Self {
        Self {
            status: ClaimStatusType::None,
            last_claim_epoch: 0,
            current_iteration: 1,
            starting_token_reserve: BigUint::zero(),
        }
    }
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone, PartialEq, Debug)]
pub struct DelegationContractData<M: ManagedTypeApi> {
    pub total_staked: BigUint<M>,
    pub delegation_contract_cap: u64,
    pub nr_nodes: u64,
    pub apy: u64,
    pub total_staked_from_ls_contract: BigUint<M>,
    pub total_undelegated_from_ls_contract: BigUint<M>,
}

#[elrond_wasm::module]
pub trait DelegationModule: crate::config::ConfigModule {
    #[only_owner]
    #[endpoint(whitelistDelegationContract)]
    fn whitelist_delegation_contract(
        &self,
        contract_address: ManagedAddress,
        total_staked: BigUint,
        delegation_contract_cap: u64,
        nr_nodes: u64,
        apy: u64,
    ) {
        require!(
            self.delegation_address(&contract_address).is_empty(),
            ERROR_ALREADY_WHITELISTED
        );

        let contract_data = DelegationContractData {
            total_staked,
            delegation_contract_cap,
            nr_nodes,
            apy,
            total_staked_from_ls_contract: BigUint::zero(),
            total_undelegated_from_ls_contract: BigUint::zero(),
        };

        self.delegation_address(&contract_address)
            .set(contract_data);
        self.delegation_addresses_list().push(&contract_address);
    }

    #[only_owner]
    #[endpoint(changeDelegationContractParams)]
    fn change_delegation_contract_params(
        &self,
        contract_address: ManagedAddress,
        total_staked: BigUint,
        delegation_contract_cap: u64,
        nr_nodes: u64,
        apy: u64,
    ) {
        require!(
            !self.delegation_address(&contract_address).is_empty(),
            ERROR_NOT_WHITELISTED
        );

        let delegation_address_mapper = self.delegation_address(&contract_address);
        let old_contract_data = delegation_address_mapper.get();
        let new_contract_data = DelegationContractData {
            total_staked,
            delegation_contract_cap,
            nr_nodes,
            apy,
            total_staked_from_ls_contract: old_contract_data.total_staked_from_ls_contract,
            total_undelegated_from_ls_contract: old_contract_data
                .total_undelegated_from_ls_contract,
        };

        delegation_address_mapper.set(new_contract_data);
    }

    // TODO - add check for available delegation space
    // Round Robin
    fn get_next_delegation_contract(&self) -> ManagedAddress<Self::Api> {
        let delegation_addresses_mapper = self.delegation_addresses_list();
        let delegation_index_mapper = self.delegation_addresses_last_index();
        let last_index = delegation_index_mapper.get();
        let new_index = if last_index >= delegation_addresses_mapper.len() {
            1
        } else {
            last_index + 1
        };

        delegation_index_mapper.set(new_index);
        self.delegation_addresses_list().get(new_index)
    }

    fn can_proceed_claim_operation(
        &self,
        new_claim_status: &mut ClaimStatus<Self::Api>,
        current_epoch: u64,
    ) {
        let old_claim_status = self.delegation_claim_status().get();
        require!(
            new_claim_status.status == ClaimStatusType::None
                || new_claim_status.status == ClaimStatusType::Pending,
            ERROR_CLAIM_START
        );
        require!(
            old_claim_status.status == ClaimStatusType::Redelegated,
            ERROR_CLAIM_REDELEGATE
        );
        require!(
            current_epoch > old_claim_status.last_claim_epoch,
            ERROR_CLAIM_EPOCH
        );

        if new_claim_status.status == ClaimStatusType::None {
            new_claim_status.status = ClaimStatusType::Pending;
            new_claim_status.last_claim_epoch = current_epoch;
            new_claim_status.starting_token_reserve = self.virtual_egld_reserve().get();
        }
    }

    #[view(getDelegationAddressesList)]
    #[storage_mapper("delegation_addresses_list")]
    fn delegation_addresses_list(&self) -> VecMapper<ManagedAddress>;

    #[view(getDelegationLastClaimEpoch)]
    #[storage_mapper("delegation_last_claim_epoch")]
    fn delegation_claim_status(&self) -> SingleValueMapper<ClaimStatus<Self::Api>>;

    #[view(getDelegationAddressesLastIndex)]
    #[storage_mapper("delegation_addresses_last_index")]
    fn delegation_addresses_last_index(&self) -> SingleValueMapper<usize>;

    #[view(getDelegationAddress)]
    #[storage_mapper("delegation_address")]
    fn delegation_address(
        &self,
        contract_address: &ManagedAddress,
    ) -> SingleValueMapper<DelegationContractData<Self::Api>>;
}
