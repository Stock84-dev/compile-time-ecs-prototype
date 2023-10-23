use std::time::Instant;

pub fn bench(h: &[f32], l: &[f32], c: &[f32]) {
    let input_len = c.len();
    let now = Instant::now();
    let mut out = vec![0u8; std::mem::size_of::<Account>() * 100 * 100 * 100];
    unsafe {
        let out_ptr = out.as_ptr() as *mut u8;
        add(h, l, c, 0, out_ptr, 100 * 100 * 100);
    }
    let elapsed_ns = now.elapsed().as_nanos();
    println!(
        "Elapsed: {} ms, {} candles/s",
        elapsed_ns / 1_000_000,
        input_len as f32 * 1_000_000_000. / elapsed_ns as f32,
    );
}
unsafe fn add(h: &[f32], l: &[f32], c: &[f32], id: usize, out: *mut u8, n_accounts: u32) {
    let n_accounts = n_accounts as usize;
    // *out = 1;
    if id > n_accounts - 1 {
        return;
    }
    // if idx >= a.len() {
    //     return;
    // }
    let stop_loss = 0.01;
    // let mut comb = id;
    // let lline = (comb % 100) as f32;
    // comb /= 100;
    // let hline = (comb % 100) as f32;
    // comb /= 100;
    // let period = comb as usize;
    let period = 11;
    let lline = 19.;
    let hline = 6.;

    let mut state = RsiState::new(period, c);
    let mut offset = state.update_at();
    let mut prev_rsi = state.update(c, offset);
    let mut prev_close = c[offset];

    offset += 1;
    let mut account = Account::new();
    for i in offset..c.len() {
        let rsi = state.update(c, i);
        account.update(
            h, l, c, i, prev_close, prev_rsi, rsi, stop_loss, hline, lline,
        );
        prev_close = c[i];
        prev_rsi = rsi;
    }
    let account_size = core::mem::size_of::<Account>();
    let acc = core::slice::from_raw_parts(&account as *const _ as *const u8, account_size);

    let out = core::slice::from_raw_parts_mut(out, n_accounts * account_size);

    out[account_size * id as usize..account_size * (id + 1) as usize].copy_from_slice(acc);
    // println!("{:#?}", account);
}

#[derive(Default, Clone, Copy)]
pub struct RsiState {
    period: usize,
    avg_gain: f32,
    avg_loss: f32,
}

impl RsiState {
    #[inline(always)]
    pub fn new(period: usize, data: &[f32]) -> Self {
        let mut avg_gain = 0.;
        let mut avg_loss = 0.;
        for i in 1..period + 1 {
            let price = data[i];
            let prev_price = data[i - 1];
            let diff = price - prev_price;
            avg_gain += (diff > 0.) as u32 as f32 * diff / period as f32;
            avg_loss -= (diff < 0.) as u32 as f32 * diff / period as f32;
        }

        Self {
            period,
            avg_gain,
            avg_loss,
        }
    }

    #[inline(always)]
    pub fn update_at(&self) -> usize {
        self.period + 1
    }

    #[inline(always)]
    pub fn update(&mut self, data: &[f32], offset: usize) -> f32 {
        let price = data[offset];
        let prev_price = data[offset - 1];
        let diff = price - prev_price;
        let last_price = data[offset - self.period];
        let last_prev_price = data[offset - self.period - 1];
        let last_diff = last_price - last_prev_price;
        // Using rolling average because it is faster, but it is prone to prcision errors
        // First remove from average to minimize floating point precision errors
        self.avg_gain -= (last_diff > 0.) as u32 as f32 * last_diff / self.period as f32;
        self.avg_loss += (last_diff < 0.) as u32 as f32 * last_diff / self.period as f32;

        self.avg_gain += (diff > 0.) as u32 as f32 * diff / self.period as f32;
        self.avg_loss -= (diff < 0.) as u32 as f32 * diff / self.period as f32;

        let mut rs = self.avg_gain / self.avg_loss;
        rs = if rs.is_nan() { 1. } else { rs };
        let rsi = 100. - (100. / (1. + rs));
        // we could clamp the value between 0 and 100, no need to bother, happens rarely
        //        assert!(rsi >= 0.);
        //        assert!(rsi <= 100.);

        return rsi;
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Account {
    pub balance: f32,
    pub entry_price: f32,
    pub max_balance: f32,
    pub max_drawdown: f32,
    pub trade_bar: u32,
    pub n_trades: u32,
    pub taker_fee: f32,
    pub slippage: f32,
}

impl Account {
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            balance: 1.,
            entry_price: 0.,
            max_drawdown: 0.,
            max_balance: 1.,
            trade_bar: 0,
            n_trades: 0,
            taker_fee: 0.00075,
            slippage: 0.00075,
        }
    }

    #[inline(always)]
    pub fn open_short(&mut self, should_open_short: bool, price: f32, fee: f32) {
        if should_open_short {
            self.balance -= self.balance * fee;
            self.entry_price = -price;
        }
    }

    #[inline(always)]
    pub fn close_short(&mut self, should_close_short: bool, price: f32, fee: f32) {
        if should_close_short {
            let entry_price_abs: f32 = self.entry_price.abs();
            let position: f32 = self.balance / self.entry_price;
            self.balance += (price - entry_price_abs) * position + price * position * fee;
            self.entry_price = 0.;
        }
    }

    #[inline(always)]
    pub fn open_long(&mut self, should_open_long: bool, price: f32, fee: f32) {
        if should_open_long {
            self.balance -= self.balance * fee;
            self.entry_price = price;
        }
    }

    #[inline(always)]
    pub fn close_long(&mut self, should_close_long: bool, price: f32, fee: f32) {
        if should_close_long {
            // even tough we know that price is abs we leave it like that so that compiler can
            // easily optimize code
            let entry_price_abs: f32 = self.entry_price.abs();
            let position: f32 = self.balance / self.entry_price;
            self.balance += (price - entry_price_abs) * position - price * position * fee;
            self.entry_price = 0.;
        }
    }

    #[inline(always)]
    pub fn closed_position(&mut self, should_close_position: bool, input_i: usize) {
        if should_close_position {
            self.max_balance = self.max_balance.max(self.balance);
            let max_drawdown = 1. - self.balance / self.max_balance;
            self.max_drawdown = self.max_drawdown.max(max_drawdown);
            self.trade_bar = input_i as u32;
            self.n_trades += 1;
        }
    }

    #[inline(always)]
    pub fn market_open_short(&mut self, should_open_short: bool, price: f32) {
        self.open_short(should_open_short, price, self.taker_fee + self.slippage);
    }

    #[inline(always)]
    pub fn market_close_short(&mut self, should_close_short: bool, price: f32) {
        self.close_short(should_close_short, price, self.taker_fee + self.slippage);
    }

    #[inline(always)]
    pub fn market_open_long(&mut self, should_open_long: bool, price: f32) {
        self.open_long(should_open_long, price, self.taker_fee + self.slippage);
    }

    #[inline(always)]
    pub fn market_close_long(&mut self, should_close_long: bool, price: f32) {
        self.close_long(should_close_long, price, self.taker_fee + self.slippage);
    }

    #[inline(always)]
    pub fn limit_open_short(&mut self, should_open_short: bool, price: f32) {
        self.open_short(should_open_short, price, 0.);
    }

    #[inline(always)]
    pub fn limit_close_short(&mut self, should_close_short: bool, price: f32) {
        self.close_short(should_close_short, price, 0.);
    }

    #[inline(always)]
    pub fn limit_open_long(&mut self, should_open_long: bool, price: f32) {
        self.open_long(should_open_long, price, 0.);
    }

    #[inline(always)]
    pub fn limit_close_long(&mut self, should_close_long: bool, price: f32) {
        self.close_long(should_close_long, price, 0.);
    }

    #[inline(always)]
    pub fn short_stopped(&mut self, stop_price: f32, high: f32) -> bool {
        let should_stop_short = self.entry_price < 0. && high > stop_price;
        self.market_close_short(should_stop_short, stop_price);
        should_stop_short
    }

    #[inline(always)]
    pub fn long_stopped(&mut self, stop_price: f32, low: f32) -> bool {
        let should_stop_long = self.entry_price > 0. && low < stop_price;
        self.market_close_long(should_stop_long, stop_price);
        should_stop_long
    }

    #[inline(always)]
    pub fn update(
        &mut self,
        high: &[f32],
        low: &[f32],
        close: &[f32],
        i: usize,
        prev_close: f32,
        prev_rsi: f32,
        rsi: f32,
        max_risk: f32,
        hline: f32,
        lline: f32,
    ) {
        let hline_condition = prev_rsi < hline && rsi >= hline;
        let lline_condition = prev_rsi >= lline && rsi < lline;
        let open_long = self.entry_price == 0. && lline_condition;
        let open_short = self.entry_price == 0. && hline_condition;
        let close_long = self.entry_price > 0. && hline_condition;
        let close_short = self.entry_price < 0. && lline_condition;
        let stop_short_price = self.entry_price * -(1. + max_risk);
        let stop_long_price = self.entry_price * (1. - max_risk);
        let stop_short = self.short_stopped(stop_short_price, high[i]);
        let stop_long = self.long_stopped(stop_long_price, low[i]);
        let close_position = close_long | close_short | stop_long | stop_short;
        self.market_open_short(open_short, close[i]);
        self.market_open_long(open_long, close[i]);
        self.market_close_long(close_long, close[i]);
        self.market_close_short(close_short, close[i]);
        self.closed_position(close_position, i);
    }
}
