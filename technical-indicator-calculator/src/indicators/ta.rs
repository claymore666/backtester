// Simple implementation of technical indicators to replace the missing ta crate
// This is a minimal implementation that provides just the interfaces needed

/// The `Next` trait is used for indicators that produce a single value
pub trait Next<T> {
    type Output;
    fn next(&mut self, input: T) -> Self::Output;
}

/// Relative Strength Index (RSI) indicator
pub struct RelativeStrengthIndex {
    period: usize,
    prev_value: Option<f64>,
    gains: Vec<f64>,
    losses: Vec<f64>,
    avg_gain: Option<f64>,
    avg_loss: Option<f64>,
    index: usize,
}

impl RelativeStrengthIndex {
    pub fn new(period: usize) -> anyhow::Result<Self> {
        if period == 0 {
            return Err(anyhow::anyhow!("Period must be greater than 0"));
        }
        
        Ok(Self {
            period,
            prev_value: None,
            gains: Vec::with_capacity(period),
            losses: Vec::with_capacity(period),
            avg_gain: None,
            avg_loss: None,
            index: 0,
        })
    }
}

impl Next<f64> for RelativeStrengthIndex {
    type Output = f64;
    
    fn next(&mut self, input: f64) -> Self::Output {
        if let Some(prev) = self.prev_value {
            let change = input - prev;
            let (gain, loss) = if change >= 0.0 {
                (change, 0.0)
            } else {
                (0.0, -change)
            };
            
            if self.index < self.period {
                // Initial period - collecting data
                self.gains.push(gain);
                self.losses.push(loss);
                
                if self.index == self.period - 1 {
                    // Calculate initial averages
                    self.avg_gain = Some(self.gains.iter().sum::<f64>() / self.period as f64);
                    self.avg_loss = Some(self.losses.iter().sum::<f64>() / self.period as f64);
                }
            } else {
                // Smooth averages
                self.avg_gain = Some((self.avg_gain.unwrap() * (self.period - 1) as f64 + gain) / self.period as f64);
                self.avg_loss = Some((self.avg_loss.unwrap() * (self.period - 1) as f64 + loss) / self.period as f64);
            }
            
            self.index += 1;
        }
        
        self.prev_value = Some(input);
        
        // Calculate RSI
        if self.index >= self.period && self.avg_gain.is_some() && self.avg_loss.is_some() {
            let avg_gain = self.avg_gain.unwrap();
            let avg_loss = self.avg_loss.unwrap();
            
            if avg_loss == 0.0 {
                100.0
            } else {
                let rs = avg_gain / avg_loss;
                100.0 - (100.0 / (1.0 + rs))
            }
        } else {
            f64::NAN
        }
    }
}

/// MACD (Moving Average Convergence Divergence) output
pub struct MacdOutput {
    pub macd: f64,
    pub signal: f64,
    pub histogram: f64,
}

/// Moving Average Convergence Divergence
pub struct MovingAverageConvergenceDivergence {
    fast_ema: ExponentialMovingAverage,
    slow_ema: ExponentialMovingAverage,
    signal_ema: ExponentialMovingAverage,
    slow_period: usize,
    signal_period: usize,
    macd_values: Vec<f64>,
    index: usize,
}

impl MovingAverageConvergenceDivergence {
    pub fn new(fast_period: usize, slow_period: usize, signal_period: usize) -> anyhow::Result<Self> {
        if fast_period >= slow_period {
            return Err(anyhow::anyhow!("Fast period must be less than slow period"));
        }
        
        if fast_period == 0 || slow_period == 0 || signal_period == 0 {
            return Err(anyhow::anyhow!("Periods must be greater than 0"));
        }
        
        Ok(Self {
            fast_ema: ExponentialMovingAverage::new(fast_period)?,
            slow_ema: ExponentialMovingAverage::new(slow_period)?,
            signal_ema: ExponentialMovingAverage::new(signal_period)?,
            slow_period,
            signal_period,
            macd_values: Vec::with_capacity(signal_period),
            index: 0,
        })
    }
}

impl Next<f64> for MovingAverageConvergenceDivergence {
    type Output = MacdOutput;
    
    fn next(&mut self, input: f64) -> Self::Output {
        let fast_ema = self.fast_ema.next(input);
        let slow_ema = self.slow_ema.next(input);
        let macd = fast_ema - slow_ema;
        
        if self.index < self.slow_period + self.signal_period - 1 {
            self.macd_values.push(macd);
        } else if self.index == self.slow_period + self.signal_period - 1 {
            self.macd_values.push(macd);
            // Initialize signal EMA with simple average
            let init_signal = self.macd_values.iter().sum::<f64>() / self.signal_period as f64;
            for _ in 0..self.signal_period {
                self.signal_ema.next(init_signal);
            }
        }
        
        let signal = if self.index >= self.slow_period + self.signal_period - 1 {
            self.signal_ema.next(macd)
        } else {
            f64::NAN
        };
        
        let histogram = macd - signal;
        self.index += 1;
        
        MacdOutput {
            macd,
            signal,
            histogram,
        }
    }
}

/// Exponential Moving Average
pub struct ExponentialMovingAverage {
    period: usize,
    alpha: f64,
    value: Option<f64>,
    index: usize,
    sum: f64,
}

impl ExponentialMovingAverage {
    pub fn new(period: usize) -> anyhow::Result<Self> {
        if period == 0 {
            return Err(anyhow::anyhow!("Period must be greater than 0"));
        }
        
        Ok(Self {
            period,
            alpha: 2.0 / (period as f64 + 1.0),
            value: None,
            index: 0,
            sum: 0.0,
        })
    }
}

impl Next<f64> for ExponentialMovingAverage {
    type Output = f64;
    
    fn next(&mut self, input: f64) -> Self::Output {
        if self.value.is_none() {
            if self.index < self.period - 1 {
                // Accumulate values for SMA
                self.sum += input;
                self.index += 1;
                return f64::NAN;
            } else {
                // Initialize EMA with SMA
                self.sum += input;
                self.value = Some(self.sum / self.period as f64);
                return self.value.unwrap();
            }
        }
        
        // Calculate EMA
        self.value = Some(input * self.alpha + self.value.unwrap() * (1.0 - self.alpha));
        self.value.unwrap()
    }
}

/// Commodity Channel Index
pub struct CommodityChannelIndex {
    period: usize,
    typical_prices: Vec<f64>,
    index: usize,
}

impl CommodityChannelIndex {
    pub fn new(period: usize) -> anyhow::Result<Self> {
        if period == 0 {
            return Err(anyhow::anyhow!("Period must be greater than 0"));
        }
        
        Ok(Self {
            period,
            typical_prices: Vec::with_capacity(period),
            index: 0,
        })
    }
}

impl Next<f64> for CommodityChannelIndex {
    type Output = f64;
    
    fn next(&mut self, typical_price: f64) -> Self::Output {
        if self.typical_prices.len() >= self.period {
            self.typical_prices.remove(0);
        }
        
        self.typical_prices.push(typical_price);
        self.index += 1;
        
        if self.typical_prices.len() < self.period {
            return f64::NAN;
        }
        
        // Calculate the SMA of typical prices
        let sma = self.typical_prices.iter().sum::<f64>() / self.period as f64;
        
        // Calculate mean deviation
        let mean_deviation = self.typical_prices.iter()
            .map(|p| (p - sma).abs())
            .sum::<f64>() / self.period as f64;
        
        if mean_deviation == 0.0 {
            return 0.0;
        }
        
        // Calculate CCI
        (typical_price - sma) / (0.015 * mean_deviation)
    }
}

/// Money Flow Index
pub struct MoneyFlowIndex {
    period: usize,
    positive_flows: Vec<f64>,
    negative_flows: Vec<f64>,
    prev_money_flow: Option<f64>,
    index: usize,
}

impl MoneyFlowIndex {
    pub fn new(period: usize) -> anyhow::Result<Self> {
        if period == 0 {
            return Err(anyhow::anyhow!("Period must be greater than 0"));
        }
        
        Ok(Self {
            period,
            positive_flows: Vec::with_capacity(period),
            negative_flows: Vec::with_capacity(period),
            prev_money_flow: None,
            index: 0,
        })
    }
}

impl Next<f64> for MoneyFlowIndex {
    type Output = f64;
    
    fn next(&mut self, money_flow: f64) -> Self::Output {
        if let Some(prev) = self.prev_money_flow {
            if self.positive_flows.len() >= self.period {
                self.positive_flows.remove(0);
                self.negative_flows.remove(0);
            }
            
            if money_flow > prev {
                self.positive_flows.push(money_flow);
                self.negative_flows.push(0.0);
            } else if money_flow < prev {
                self.positive_flows.push(0.0);
                self.negative_flows.push(money_flow);
            } else {
                self.positive_flows.push(0.0);
                self.negative_flows.push(0.0);
            }
        }
        
        self.prev_money_flow = Some(money_flow);
        self.index += 1;
        
        if self.index <= self.period {
            return f64::NAN;
        }
        
        let positive_sum = self.positive_flows.iter().sum::<f64>();
        let negative_sum = self.negative_flows.iter().sum::<f64>();
        
        if positive_sum + negative_sum == 0.0 {
            return 50.0;
        }
        
        let money_ratio = positive_sum / negative_sum;
        100.0 - (100.0 / (1.0 + money_ratio))
    }
}

/// Rate of Change
pub struct RateOfChange {
    period: usize,
    prices: Vec<f64>,
    index: usize,
}

impl RateOfChange {
    pub fn new(period: usize) -> anyhow::Result<Self> {
        if period == 0 {
            return Err(anyhow::anyhow!("Period must be greater than 0"));
        }
        
        Ok(Self {
            period,
            prices: Vec::with_capacity(period + 1),
            index: 0,
        })
    }
}

impl Next<f64> for RateOfChange {
    type Output = f64;
    
    fn next(&mut self, price: f64) -> Self::Output {
        self.prices.push(price);
        
        if self.prices.len() > self.period + 1 {
            self.prices.remove(0);
        }
        
        self.index += 1;
        
        if self.prices.len() <= self.period {
            return f64::NAN;
        }
        
        let old_price = self.prices[0];
        
        if old_price == 0.0 {
            return f64::NAN;
        }
        
        (price - old_price) / old_price * 100.0
    }
}

/// Percentage Price Oscillator output
pub struct PpoOutput {
    pub ppo: f64,
    pub signal: f64,
    pub histogram: f64,
}

/// Percentage Price Oscillator
pub struct PercentagePriceOscillator {
    fast_ema: ExponentialMovingAverage,
    slow_ema: ExponentialMovingAverage,
    signal_ema: ExponentialMovingAverage,
    slow_period: usize,
    signal_period: usize,
    ppo_values: Vec<f64>,
    index: usize,
}

impl PercentagePriceOscillator {
    pub fn new(fast_period: usize, slow_period: usize, signal_period: usize) -> anyhow::Result<Self> {
        if fast_period >= slow_period {
            return Err(anyhow::anyhow!("Fast period must be less than slow period"));
        }
        
        if fast_period == 0 || slow_period == 0 || signal_period == 0 {
            return Err(anyhow::anyhow!("Periods must be greater than 0"));
        }
        
        Ok(Self {
            fast_ema: ExponentialMovingAverage::new(fast_period)?,
            slow_ema: ExponentialMovingAverage::new(slow_period)?,
            signal_ema: ExponentialMovingAverage::new(signal_period)?,
            slow_period,
            signal_period,
            ppo_values: Vec::with_capacity(signal_period),
            index: 0,
        })
    }
}

impl Next<f64> for PercentagePriceOscillator {
    type Output = PpoOutput;
    
    fn next(&mut self, input: f64) -> Self::Output {
        let fast_ema = self.fast_ema.next(input);
        let slow_ema = self.slow_ema.next(input);
        
        let ppo = if slow_ema != 0.0 {
            (fast_ema - slow_ema) / slow_ema * 100.0
        } else {
            0.0
        };
        
        if self.index < self.slow_period + self.signal_period - 1 {
            self.ppo_values.push(ppo);
        } else if self.index == self.slow_period + self.signal_period - 1 {
            self.ppo_values.push(ppo);
            // Initialize signal EMA with simple average
            let init_signal = self.ppo_values.iter().sum::<f64>() / self.signal_period as f64;
            for _ in 0..self.signal_period {
                self.signal_ema.next(init_signal);
            }
        }
        
        let signal = if self.index >= self.slow_period + self.signal_period - 1 {
            self.signal_ema.next(ppo)
        } else {
            f64::NAN
        };
        
        let histogram = ppo - signal;
        self.index += 1;
        
        PpoOutput {
            ppo,
            signal,
            histogram,
        }
    }
}

/// Standard Deviation
pub struct StandardDeviation {
    period: usize,
    values: Vec<f64>,
}

impl StandardDeviation {
    pub fn new(period: usize) -> anyhow::Result<Self> {
        if period == 0 {
            return Err(anyhow::anyhow!("Period must be greater than 0"));
        }
        
        Ok(Self {
            period,
            values: Vec::with_capacity(period),
        })
    }
}

impl Next<f64> for StandardDeviation {
    type Output = f64;
    
    fn next(&mut self, input: f64) -> Self::Output {
        if self.values.len() >= self.period {
            self.values.remove(0);
        }
        
        self.values.push(input);
        
        if self.values.len() < self.period {
            return f64::NAN;
        }
        
        let mean = self.values.iter().sum::<f64>() / self.period as f64;
        let variance = self.values.iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>() / self.period as f64;
        
        variance.sqrt()
    }
}

/// Average True Range
pub struct AverageTrueRange {
    period: usize,
    tr_values: Vec<f64>,
    atr: Option<f64>,
    index: usize,
}

impl AverageTrueRange {
    pub fn new(period: usize) -> anyhow::Result<Self> {
        if period == 0 {
            return Err(anyhow::anyhow!("Period must be greater than 0"));
        }
        
        Ok(Self {
            period,
            tr_values: Vec::with_capacity(period),
            atr: None,
            index: 0,
        })
    }
}

impl Next<f64> for AverageTrueRange {
    type Output = f64;
    
    fn next(&mut self, tr: f64) -> Self::Output {
        if self.index < self.period {
            self.tr_values.push(tr);
            
            if self.index == self.period - 1 {
                // Initialize ATR with simple average
                self.atr = Some(self.tr_values.iter().sum::<f64>() / self.period as f64);
            }
        } else {
            // Apply smoothing formula: ATR = (prev_ATR * (period-1) + TR) / period
            self.atr = Some((self.atr.unwrap() * (self.period - 1) as f64 + tr) / self.period as f64);
        }
        
        self.index += 1;
        
        self.atr.unwrap_or(f64::NAN)
    }
}

// Make the module public
pub mod indicators {
    pub use super::*;
}
