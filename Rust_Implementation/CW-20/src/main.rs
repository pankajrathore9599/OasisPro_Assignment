use cosmwasm_std::{
    to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier, StdError, StdResult,
    Storage, Uint128,
};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};
use cw20::{Balance, Cw20Coin, Cw20CoinVerified, Expiration, MinterResponse, TokenInfoResponse};

pub static BALANCES: &[u8] = b"balances";
pub static TOTAL_SUPPLY: &[u8] = b"total_supply";
pub static MINTER: &[u8] = b"minter";
pub static CAP: &[u8] = b"cap";
pub static FROZEN_BALANCES: &[u8] = b"frozen_balances";

#[derive(Default)]
pub struct State {
    pub balances: Singleton<dyn Storage>,
    pub total_supply: Singleton<dyn Storage>,
    pub minter: Singleton<dyn Storage>,
    pub cap: Singleton<dyn Storage>,
    pub frozen_balances: Singleton<dyn Storage>,
}

impl State {
    pub fn new(storage: &mut dyn Storage) -> Self {
        Self {
            balances: singleton(storage, BALANCES),
            total_supply: singleton(storage, TOTAL_SUPPLY),
            minter: singleton(storage, MINTER),
            cap: singleton(storage, CAP),
            frozen_balances: singleton(storage, FROZEN_BALANCES),
        }
    }

    pub fn update_cap(&mut self, new_cap: Uint128) {
        self.cap.save(&new_cap);
    }

    pub fn cap(&self) -> Uint128 {
        self.cap.load()
    }

    pub fn update_minter(&mut self, minter: String, cap: Uint128) {
        let new_minter = MinterResponse {
            minter,
            cap: Some(cap),
        };
        self.minter.save(&new_minter);
        self.update_cap(cap);
    }

    pub fn minter(&self) -> MinterResponse {
        self.minter.load()
    }

    pub fn mint(&mut self, recipient: &str, amount: Uint128) -> StdResult<()> {
        let minter = self.minter();
        if minter.cap.map_or(false, |cap| cap < self.total_supply()? + amount) {
            return Err(StdError::generic_err("Cannot mint more tokens than the minter cap"));
        }
        self.balances.update(recipient.as_bytes(), |balance| -> StdResult<_> {
            let new_balance = balance.unwrap_or_default() + amount;
            Ok(Some(new_balance))
        })?;
        self.total_supply.update(|supply| Ok(Some(supply.unwrap_or_default() + amount)))?;
        Ok(())
    }

    pub fn transfer(
        &mut self,
        sender: &str,
        recipient: &str,
        amount: Uint128,
    ) -> StdResult<()> {
        if self.is_frozen(sender)? {
            return Err(StdError::generic_err("Cannot transfer from a frozen account"));
        }
        let sender_balance = self.balance(sender)?;
        if sender == recipient {
            return Ok(());
        }
        if amount.is_zero() {
            return Ok(());
        }
        if sender_balance < amount {
            return Err(StdError::generic_err("Cannot send more tokens than you have"));
        }
        self.balances.update(sender.as_bytes(), |balance| -> StdResult<_> {
            let new_balance = balance.unwrap_or_default() - amount;
            Ok(Some(new_balance))
        })?;
        self.balances.update(recipient.as_bytes(), |balance| -> StdResult<_> {
            let new_balance = balance.unwrap_or_default() + amount;
            if self.cap().map_or(false, |cap| new_balance > cap) {
                return Err(StdError::generic_err("Cannot hold more tokens than the cap"));
            }
            Ok(Some(new_balance))
        })?;
        Ok(())
    }

    pub fn balance(&self, address: &str) -> StdResult<Uint128> {
        Ok(self.balances.may_load(address.as_bytes())?.unwrap_or_default())
    }

    pub fn total_supply(&self) -> StdResult<Uint128> {
        Ok(self.total_supply.may_load()?.unwrap_or_default())
    }

    pub fn token_info(&self) -> StdResult<TokenInfoResponse> {
        Ok(TokenInfoResponse {
            name: "My Token".to_string(),
            symbol: "MTK".to_string(),
            decimals: 18,
            total_supply: self.total_supply()?,
        })
    }

    pub fn minter_allowed(&self, sender: &str) -> bool {
        let minter = self.minter();
        minter.minter == sender && minter.cap.is_some()
    }

    pub fn is_frozen(&self, address: &str) -> StdResult<bool> {
        Ok(self.frozen_balances.may_load(address.as_bytes())?.unwrap_or_default())
    }

    pub fn freeze(&mut self, address: &str) -> StdResult<()> {
        self.frozen_balances.save(address.as_bytes(), &true)?;
        Ok(())
    }

    pub fn unfreeze(&mut self, address: &str) -> StdResult<()> {
        self.frozen_balances.remove(address.as_bytes());
        Ok(())
    }

    pub fn execute(
        &mut self,
        api: &dyn Api,
        env: &Env,
        msg: &HandleMsg,
    ) -> Result<HandleResponse, StdError> {
        match msg {
            HandleMsg::Transfer { recipient, amount } => {
                let sender_address = env.message.sender.clone();
                let recipient_address = api.addr_validate(recipient)?;
                self.transfer(
                    &sender_address.to_string(),
                    &recipient_address.to_string(),
                    amount.clone(),
                )?;
                Ok(HandleResponse::default())
            }
            HandleMsg::Mint { recipient, amount } => {
                if !self.minter_allowed(&env.message.sender) {
                    return Err(StdError::generic_err("Unauthorized"));
                }
                let recipient_address = api.addr_validate(recipient)?;
                self.mint(&recipient_address.to_string(), amount.clone())?;
                Ok(HandleResponse::default())
            }
            HandleMsg::UpdateMinter { minter, cap } => {
                if !self.minter_allowed(&env.message.sender) {
                    return Err(StdError::generic_err("Unauthorized"));
                }
                self.update_minter(minter.clone(), cap.unwrap_or_default());
                Ok(HandleResponse::default())
            }
            HandleMsg::Freeze { address } => {
                if !self.minter_allowed(&env.message.sender) {
                    return Err(StdError::generic_err("Unauthorized"));
                }
                let address = api.addr_validate(address)?;
                self.freeze(&address.to_string())?;
                Ok(HandleResponse::default())
            }
            HandleMsg::Unfreeze { address } => {
                if !self.minter_allowed(&env.message.sender) {
                    return Err(StdError::generic_err("Unauthorized"));
                }
                let address = api.addr_validate(address)?;
                self.unfreeze(&address.to_string())?;
                Ok(HandleResponse::
                    default())
                }
            }
        }
    }
    
