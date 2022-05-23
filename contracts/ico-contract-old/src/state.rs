use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Decimal, Env, StdResult, Storage, Uint128};
use cw_storage_plus::{Item, Map};

use crate::msg::IcoStage;

pub const OWNER: Item<Addr> = Item::new("owner");
pub const PRICE_DENOM: Item<String> = Item::new("price_denom");
pub const TOKENADDR: Item<Addr> = Item::new("token_addr");
pub const WHITELIST: Map<Addr, ()> = Map::new("whitelisted");
pub const BALANCES: Map<Addr, Uint128> = Map::new("balances");
pub const REVENUE_DISTRIBUTION: Map<Addr, Decimal> = Map::new("revenue_distribution");

pub trait SelfLoadAndSave
where
    Self: Sized,
{
    fn load(storage: &dyn Storage) -> StdResult<Self>;
    fn save(&self, storage: &mut dyn Storage) -> StdResult<()>;
}

pub trait CoinSupply {
    fn record_sale(&mut self, amount_sold: Uint128);
    fn remains(&self) -> Uint128;
    fn price(&self) -> Uint128;
    fn price_for_amount(&self, amount: Uint128) -> Uint128;
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct PrivateSaleCoinSupply {
    pub total_amount: Uint128,
    pub coins_sold: Uint128,
    pub coin_price: Uint128,
}

impl PrivateSaleCoinSupply {
    pub fn new(total_amount: Uint128, coin_price: Uint128) -> Self {
        Self {
            total_amount,
            coins_sold: Uint128::zero(),
            coin_price,
        }
    }
}

impl CoinSupply for PrivateSaleCoinSupply {
    fn record_sale(&mut self, amount_sold: Uint128) {
        self.coins_sold += amount_sold;
    }

    fn remains(&self) -> Uint128 {
        self.total_amount - self.coins_sold
    }

    fn price(&self) -> Uint128 {
        self.coin_price
    }

    fn price_for_amount(&self, amount: Uint128) -> Uint128 {
        self.coin_price * amount
    }
}

impl SelfLoadAndSave for PrivateSaleCoinSupply {
    fn load(storage: &dyn Storage) -> StdResult<Self> {
        PRIVATE_COIN_SUPPLY.load(storage)
    }

    fn save(&self, storage: &mut dyn Storage) -> StdResult<()> {
        PRIVATE_COIN_SUPPLY.save(storage, self)
    }
}

pub const PRIVATE_COIN_SUPPLY: Item<PrivateSaleCoinSupply> = Item::new("private_coin");

// separate stuct for internal data, so we dont need to store additional info
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct PublicSaleCoinSupplyData {
    pub total_amount: Uint128,
    pub coins_sold: Uint128,
    pub coin_price_start: Uint128,
    pub coin_price_end: Uint128,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct PublicSaleCoinSupply {
    pub inherited: Uint128,
    pub data: PublicSaleCoinSupplyData,
}

impl PublicSaleCoinSupply {
    const PRECISION_MULTIPLIER: u128 = 1_000_000_u128;

    pub fn from_data(data: PublicSaleCoinSupplyData) -> Self {
        Self {
            inherited: Uint128::zero(),
            data,
        }
    }

    pub fn new(total_amount: Uint128, coin_price_start: Uint128, coin_price_end: Uint128) -> Self {
        Self {
            inherited: Uint128::zero(),
            data: PublicSaleCoinSupplyData {
                total_amount,
                coins_sold: Uint128::zero(),
                coin_price_start,
                coin_price_end,
            },
        }
    }

    fn inherit_private(&mut self, private_sale: &PrivateSaleCoinSupply) {
        self.inherited = private_sale.remains();
    }

    fn price_at(&self, coins_sold: Uint128) -> Uint128 {
        let pm = Uint128::from(Self::PRECISION_MULTIPLIER);
        self.data.coin_price_start * pm
            + (self.data.coin_price_end - self.data.coin_price_start)
                * pm
                * Decimal::from_ratio(coins_sold, self.data.total_amount + self.inherited)
    }
}

impl CoinSupply for PublicSaleCoinSupply {
    fn record_sale(&mut self, amount_sold: Uint128) {
        self.data.coins_sold += amount_sold;
    }

    fn remains(&self) -> Uint128 {
        self.data.total_amount + self.inherited - self.data.coins_sold
    }

    fn price(&self) -> Uint128 {
        self.price_at(self.data.coins_sold) / Uint128::from(Self::PRECISION_MULTIPLIER)
    }

    fn price_for_amount(&self, amount: Uint128) -> Uint128 {
        let average = Decimal::from_ratio(
            self.price_at(self.data.coins_sold) + self.price_at(self.data.coins_sold + amount),
            2_u128,
        );
        average * amount / Uint128::from(Self::PRECISION_MULTIPLIER)
    }
}

impl SelfLoadAndSave for PublicSaleCoinSupply {
    fn load(storage: &dyn Storage) -> StdResult<Self> {
        let private_sale = PRIVATE_COIN_SUPPLY.load(storage)?;
        let mut public_sale = PublicSaleCoinSupply::from_data(PUBLIC_COIN_SUPPLY_DATA.load(storage)?);
        public_sale.inherit_private(&private_sale);
        Ok(public_sale)
    }

    fn save(&self, storage: &mut dyn Storage) -> StdResult<()> {
        PUBLIC_COIN_SUPPLY_DATA.save(storage, &self.data)
    }
}

pub const PUBLIC_COIN_SUPPLY_DATA: Item<PublicSaleCoinSupplyData> = Item::new("public_sale_data");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct IcoTimer {
    start_time: Option<u64>,
    private_end: u64,
    public_end: u64,
}

impl IcoTimer {
    pub fn new(private_duration: u64, public_duration: u64) -> Self {
        Self {
            start_time: None,
            private_end: private_duration,
            public_end: public_duration,
        }
    }

    pub fn get_start_ime(&self) ->Option<u64> {
        self.start_time 
    }

    pub fn get_private_end(&self) ->u64 {
        self.private_end 
    }

    pub fn get_public_end(&self) ->u64 {
        self.public_end 
    }

    pub fn start(&mut self, env: &Env) {
        let start_time = env.block.time.seconds();
        self.start_time = Some(start_time);
        self.private_end += start_time;
        self.public_end += self.private_end;
    }

    pub fn end(&mut self) {
        self.private_end = 0;
        self.public_end = 0;
    }

    pub fn current_stage(&self, env: &Env) -> IcoStage {
        let start_time = match self.start_time {
            Some(t) => t,
            None => return IcoStage::NotStarted,
        };
        let curr = env.block.time.seconds();
        if curr < start_time {
            IcoStage::NotStarted
        } else if curr >= start_time && curr < self.private_end {
            IcoStage::PrivateSale
        } else if curr >= self.private_end && curr < self.public_end {
            IcoStage::PublicSale
        } else {
            IcoStage::Ended
        }
    }

    pub fn move_to_next_stage(&mut self, env: &Env) {
        match self.current_stage(env) {
            IcoStage::NotStarted | IcoStage::Ended => {}
            // setting `now` as private end and moving public end by the delta
            IcoStage::PrivateSale => {
                let new_pivate_end = env.block.time.seconds();
                // this is safe bacause in PrivateSale state self.private_end is in the future
                let delta = self.private_end - new_pivate_end;
                let new_public_end = self.public_end - delta;

                self.private_end = new_pivate_end;
                self.public_end = new_public_end;
            }
            // after public sale Ico ends
            IcoStage::PublicSale => self.end(),
        }
    }
}

pub const TIMER: Item<IcoTimer> = Item::new("ico_timer");
