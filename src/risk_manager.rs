use crate::{OptionSide, Contract};

pub struct RiskManager {
    risk_margin: f64,  // Safety margin (e.g., 1.2 = 20% extra margin)
}

#[derive(Debug, Clone)]
pub struct RiskMetrics {
    pub position_risk: f64,      // Risk for a single position in USD
    pub total_risk_exposure: f64, // Total portfolio risk in USD
    pub available_collateral: f64, // Available collateral after existing positions
    pub max_quantity: f64,        // Maximum quantity for new position
}

#[derive(Debug, Clone)]
pub struct PositionRisk {
    pub max_loss: f64,           // Maximum possible loss
    pub expected_loss: f64,      // Expected loss based on probability
    pub margin_required: f64,    // Collateral required
}

impl RiskManager {
    pub fn new(risk_margin: f64) -> Self {
        Self { risk_margin }
    }
    
    /// Calculate risk for a single option position
    pub fn calculate_position_risk(
        &self,
        side: &OptionSide,
        strike: f64,
        premium: f64,
        quantity: f64,
        spot_price: f64,
        iv: f64,
        time_to_expiry: f64,
        risk_free_rate: f64,
    ) -> PositionRisk {
        match side {
            OptionSide::Put => {
                // For put seller: max loss = strike - premium received (if BTC goes to 0)
                let max_loss_per_contract = strike - premium;
                let max_loss = max_loss_per_contract * quantity;
                
                // Calculate probability of being in the money using Black-Scholes N(d2)
                let d2 = calculate_d2(spot_price, strike, risk_free_rate, iv, time_to_expiry);
                let prob_itm = normal_cdf(-d2); // Probability put is ITM
                
                // Expected loss = probability weighted potential loss
                let moneyness = (strike - spot_price).max(0.0);
                let expected_loss = prob_itm * moneyness * quantity;
                
                // Margin required with safety factor
                let margin_required = max_loss * self.risk_margin;
                
                PositionRisk {
                    max_loss,
                    expected_loss,
                    margin_required,
                }
            }
            OptionSide::Call => {
                // For call seller: theoretically unlimited loss, but we cap it
                // Use 3x current price as reasonable worst case
                let max_price_move = spot_price * 3.0;
                let max_loss_per_contract = (max_price_move - strike).max(0.0) - premium;
                let max_loss = max_loss_per_contract * quantity;
                
                // Calculate probability of being in the money
                let d2 = calculate_d2(spot_price, strike, risk_free_rate, iv, time_to_expiry);
                let prob_itm = normal_cdf(d2); // Probability call is ITM
                
                // Expected loss based on current moneyness and probability
                let moneyness = (spot_price - strike).max(0.0);
                let expected_loss = prob_itm * moneyness * quantity * 1.5; // 1.5x for upside risk
                
                // Margin required with safety factor
                let margin_required = max_loss * self.risk_margin;
                
                PositionRisk {
                    max_loss,
                    expected_loss,
                    margin_required,
                }
            }
        }
    }
    
    /// Calculate total portfolio risk from existing contracts
    pub fn calculate_portfolio_risk(
        &self,
        contracts: &[Contract],
        spot_price: f64,
        risk_free_rate: f64,
        iv_oracle: &dyn Fn(&str, f64, &str) -> Option<f64>,
    ) -> f64 {
        let mut total_margin_required = 0.0;
        let current_time = chrono::Utc::now().timestamp();
        
        for contract in contracts {
            // Skip expired contracts
            if contract.expires <= current_time {
                continue;
            }
            
            let time_to_expiry = (contract.expires - current_time) as f64 / (365.0 * 24.0 * 60.0 * 60.0);
            
            // Get IV for this specific contract
            let side_str = match contract.side {
                OptionSide::Call => "C",
                OptionSide::Put => "P",
            };
            let expire_timestamp_ms = (contract.expires * 1000).to_string();
            let iv = iv_oracle(side_str, contract.strike_price, &expire_timestamp_ms)
                .unwrap_or(0.4); // Default IV if not found
            
            let position_risk = self.calculate_position_risk(
                &contract.side,
                contract.strike_price,
                contract.premium,
                contract.quantity,
                spot_price,
                iv,
                time_to_expiry,
                risk_free_rate,
            );
            
            total_margin_required += position_risk.margin_required;
        }
        
        total_margin_required
    }
    
    /// Calculate maximum quantity for a new position considering risk
    /// available_collateral_usd is already net of existing risk exposure
    pub fn calculate_max_quantity(
        &self,
        side: &OptionSide,
        strike: f64,
        premium: f64,
        spot_price: f64,
        iv: f64,
        time_to_expiry: f64,
        risk_free_rate: f64,
        available_collateral_usd: f64,
        _existing_risk_usd: f64, // Not used since available_collateral_usd is already net
    ) -> f64 {
        // Calculate risk for 1 contract
        let unit_risk = self.calculate_position_risk(
            side,
            strike,
            premium,
            1.0, // 1 contract
            spot_price,
            iv,
            time_to_expiry,
            risk_free_rate,
        );
        
        println!("🔢 Max Quantity Calculation:");
        println!("   Unit risk - max_loss: ${:.2}", unit_risk.max_loss);
        println!("   Unit risk - margin_required: ${:.2}", unit_risk.margin_required);
        println!("   Available collateral: ${:.2}", available_collateral_usd);
        
        // available_collateral_usd is already calculated as total_collateral - existing_risk
        if available_collateral_usd <= 0.0 || unit_risk.margin_required <= 0.0 {
            println!("   Max quantity = 0.0 (insufficient collateral or zero margin)");
            return 0.0;
        }
        
        // Maximum quantity based on margin requirements
        let max_quantity = available_collateral_usd / unit_risk.margin_required;
        println!("   Calculated max quantity: {:.8}", max_quantity);
        
        // Ensure we don't exceed reasonable position limits
        max_quantity.min(1000.0) // Cap at 1000 contracts per position
    }
}

// Black-Scholes helper functions
fn calculate_d1(s: f64, k: f64, r: f64, sigma: f64, t: f64) -> f64 {
    ((s / k).ln() + (r + sigma * sigma / 2.0) * t) / (sigma * t.sqrt())
}

fn calculate_d2(s: f64, k: f64, r: f64, sigma: f64, t: f64) -> f64 {
    calculate_d1(s, k, r, sigma, t) - sigma * t.sqrt()
}

fn normal_cdf(x: f64) -> f64 {
    // Approximation of cumulative normal distribution
    let abs_x = x.abs();
    let k = 1.0 / (1.0 + 0.2316419 * abs_x);
    let k2 = k * k;
    let k3 = k2 * k;
    let k4 = k3 * k;
    let _k5 = k4 * k;
    
    let a1 = 0.319381530;
    let a2 = -0.356563782;
    let a3 = 1.781477937;
    let a4 = -1.821255978;
    let a5 = 1.330274429;
    
    let norm_pdf = (1.0 / (2.0 * std::f64::consts::PI).sqrt()) * (-0.5 * x * x).exp();
    let poly = k * (a1 + k * (a2 + k * (a3 + k * (a4 + k * a5))));
    
    if x >= 0.0 {
        1.0 - norm_pdf * poly
    } else {
        norm_pdf * poly
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_put_risk_calculation() {
        let risk_manager = RiskManager::new(1.2); // 20% margin
        
        let risk = risk_manager.calculate_position_risk(
            &OptionSide::Put,
            100000.0,  // strike
            1000.0,    // premium
            1.0,       // quantity
            110000.0,  // spot price
            0.5,       // IV
            0.0833,    // 30 days
            0.05,      // risk free rate
        );
        
        assert_eq!(risk.max_loss, 99000.0); // strike - premium
        assert!(risk.expected_loss < risk.max_loss);
        assert_eq!(risk.margin_required, 99000.0 * 1.2);
    }
}