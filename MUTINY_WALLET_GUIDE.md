# Mutiny Wallet Module Guide

Bitcoin Testnet (Mutinynet) 지갑 잔고 및 트랜잭션 조회 모듈

## 📋 목차
- [설치 방법](#설치-방법)
- [기본 사용법](#기본-사용법)
- [API 레퍼런스](#api-레퍼런스)
- [예제 코드](#예제-코드)
- [네트워크 정보](#네트워크-정보)

## 설치 방법

### 현재 프로젝트에서 사용
```rust
// 이미 프로젝트에 포함되어 있음
use btc_options_api::{MutinyWallet, Network};
```

### 다른 Rust 프로젝트에서 사용
```toml
# Cargo.toml에 추가
[dependencies]
# 로컬 경로
btc_options_api = { path = "../btc-option-manager" }

# 또는 Git 저장소 (push 후)
# btc_options_api = { git = "https://github.com/yourusername/btc-option-manager" }

tokio = { version = "1", features = ["full"] }
```

## 기본 사용법

```rust
use btc_options_api::{MutinyWallet, Network};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 지갑 객체 생성
    let wallet = MutinyWallet::new(Network::Signet);
    
    // 2. 잔고 조회
    let balance = wallet.get_wallet_balance("tb1q...").await?;
    println!("잔고: {} sats", balance.total_balance);
    
    Ok(())
}
```

## API 레퍼런스

### 1. `MutinyWallet::new(network)`
지갑 객체를 생성합니다.

**Parameters:**
- `network`: `Network::Signet` (Mutinynet), `Network::Testnet`, `Network::Mainnet`

**Returns:** `MutinyWallet` 인스턴스

```rust
let wallet = MutinyWallet::new(Network::Signet);
```

---

### 2. `get_wallet_balance(address)`
지정된 주소의 지갑 잔고를 조회합니다.

**Parameters:**
- `address`: Bitcoin 주소 문자열

**Returns:** `Result<WalletBalance, MutinyWalletError>`

**Response Structure:**
```rust
WalletBalance {
    address: String,              // 조회한 주소
    confirmed_balance: u64,       // 확인된 잔고 (satoshis)
    unconfirmed_balance: u64,     // 미확인 잔고 (satoshis)
    total_balance: u64,           // 총 잔고 (satoshis)
    confirmed_utxo_count: u64,    // 확인된 UTXO 개수
    unconfirmed_utxo_count: u64,  // 미확인 UTXO 개수
    total_utxo_count: u64,        // 총 UTXO 개수
}
```

**Example:**
```rust
let balance = wallet.get_wallet_balance("tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx").await?;
println!("확인된 잔고: {} sats", balance.confirmed_balance);
println!("미확인 잔고: {} sats", balance.unconfirmed_balance);
println!("총 잔고: {} BTC", MutinyWallet::satoshis_to_btc(balance.total_balance));
```

---

### 3. `get_address_utxos(address)`
지정된 주소의 UTXO(미사용 트랜잭션 출력) 목록을 조회합니다.

**Parameters:**
- `address`: Bitcoin 주소 문자열

**Returns:** `Result<Vec<Utxo>, MutinyWalletError>`

**Response Structure:**
```rust
Utxo {
    txid: String,        // 트랜잭션 ID
    vout: u32,           // 출력 인덱스
    value: u64,          // 금액 (satoshis)
    status: UtxoStatus {
        confirmed: bool,
        block_height: Option<u64>,
        block_hash: Option<String>,
        block_time: Option<u64>,
    }
}
```

**Example:**
```rust
let utxos = wallet.get_address_utxos("tb1q...").await?;
for utxo in utxos {
    println!("TXID: {}, Value: {} sats", utxo.txid, utxo.value);
}
```

---

### 4. `get_address_transactions(address)`
지정된 주소의 트랜잭션 목록을 조회합니다.

**Parameters:**
- `address`: Bitcoin 주소 문자열

**Returns:** `Result<Vec<Transaction>, MutinyWalletError>`

**Response Structure:**
```rust
Transaction {
    txid: String,              // 트랜잭션 ID
    version: u32,              // 버전
    locktime: u32,             // 락타임
    vin: Vec<Vin>,            // 입력 목록
    vout: Vec<Vout>,          // 출력 목록
    size: u32,                // 크기 (bytes)
    weight: u32,              // Weight
    fee: u64,                 // 수수료 (satoshis)
    status: TxStatus {
        confirmed: bool,
        block_height: Option<u64>,
        block_hash: Option<String>,
        block_time: Option<u64>,
    }
}
```

**Example:**
```rust
let transactions = wallet.get_address_transactions("tb1q...").await?;
for tx in transactions {
    println!("TX: {}, Fee: {} sats, Confirmed: {}", 
        tx.txid, tx.fee, tx.status.confirmed);
}
```

---

### 5. `get_transaction(txid)`
특정 트랜잭션의 상세 정보를 조회합니다.

**Parameters:**
- `txid`: 트랜잭션 ID 문자열

**Returns:** `Result<Transaction, MutinyWalletError>`

**Example:**
```rust
let tx = wallet.get_transaction("4d74938e7e13fae143f944ceee19f1ef48e8452c3b2447ca432ff1275da3ffdd").await?;
println!("수수료: {} sats", tx.fee);
println!("확인 여부: {}", tx.status.confirmed);
```

---

### 6. 유틸리티 함수

#### `satoshis_to_btc(satoshis)`
Satoshi를 BTC로 변환합니다.

```rust
let btc = MutinyWallet::satoshis_to_btc(100_000_000);  // 1.0 BTC
```

#### `btc_to_satoshis(btc)`
BTC를 Satoshi로 변환합니다.

```rust
let sats = MutinyWallet::btc_to_satoshis(0.001);  // 100,000 sats
```

## 예제 코드

### 예제 1: 간단한 잔고 조회
```rust
use btc_options_api::{MutinyWallet, Network};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let wallet = MutinyWallet::new(Network::Signet);
    let address = "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx";
    
    let balance = wallet.get_wallet_balance(address).await?;
    println!("잔고: {} BTC", MutinyWallet::satoshis_to_btc(balance.total_balance));
    
    Ok(())
}
```

### 예제 2: 여러 주소 모니터링
```rust
use btc_options_api::{MutinyWallet, Network};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let wallet = MutinyWallet::new(Network::Signet);
    
    let addresses = vec![
        "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx",
        "tb1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gdcccefvpysxf3q0sl5k7",
    ];
    
    let mut total = 0u64;
    for address in addresses {
        if let Ok(balance) = wallet.get_wallet_balance(address).await {
            total += balance.total_balance;
            println!("{}: {} sats", address, balance.total_balance);
        }
    }
    
    println!("총합: {} BTC", MutinyWallet::satoshis_to_btc(total));
    
    Ok(())
}
```

### 예제 3: 트랜잭션 확인 체크
```rust
use btc_options_api::{MutinyWallet, Network};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let wallet = MutinyWallet::new(Network::Signet);
    let txid = "4d74938e7e13fae143f944ceee19f1ef48e8452c3b2447ca432ff1275da3ffdd";
    
    match wallet.get_transaction(txid).await {
        Ok(tx) => {
            if tx.status.confirmed {
                println!("✅ 트랜잭션 확인됨");
                if let Some(height) = tx.status.block_height {
                    println!("블록 높이: {}", height);
                }
            } else {
                println!("⏳ 확인 대기중...");
            }
        }
        Err(e) => println!("트랜잭션을 찾을 수 없음: {}", e),
    }
    
    Ok(())
}
```

### 예제 4: UTXO 분석
```rust
use btc_options_api::{MutinyWallet, Network};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let wallet = MutinyWallet::new(Network::Signet);
    let address = "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx";
    
    let utxos = wallet.get_address_utxos(address).await?;
    
    // UTXO 분석
    let total_value: u64 = utxos.iter().map(|u| u.value).sum();
    let dust_utxos = utxos.iter().filter(|u| u.value < 10000).count();
    
    println!("총 UTXO: {} 개", utxos.len());
    println!("총 가치: {} sats", total_value);
    println!("소액 UTXO (< 10k sats): {} 개", dust_utxos);
    
    // 가장 큰 UTXO
    if let Some(max_utxo) = utxos.iter().max_by_key(|u| u.value) {
        println!("최대 UTXO: {} sats (txid: {})", max_utxo.value, max_utxo.txid);
    }
    
    Ok(())
}
```

## 네트워크 정보

### Mutinynet (Signet)
- **네트워크 타입**: Bitcoin Signet (테스트넷)
- **API 엔드포인트**: `https://mutinynet.com/api`
- **블록 타임**: 30초
- **Explorer**: https://mutinynet.com
- **Faucet**: 사용 가능

### 지원 네트워크
- `Network::Signet` - Mutinynet (추천)
- `Network::Testnet` - Bitcoin Testnet
- `Network::Mainnet` - Bitcoin Mainnet

## 에러 처리

```rust
use btc_options_api::{MutinyWallet, Network, MutinyWalletError};

match wallet.get_wallet_balance("invalid_address").await {
    Ok(balance) => println!("성공: {} sats", balance.total_balance),
    Err(MutinyWalletError::NetworkError(e)) => println!("네트워크 오류: {}", e),
    Err(MutinyWalletError::ParseError(e)) => println!("파싱 오류: {}", e),
    Err(MutinyWalletError::ApiError(e)) => println!("API 오류: {}", e),
    Err(e) => println!("기타 오류: {}", e),
}
```

## 주의사항

1. **비동기 처리**: 모든 API 호출은 `async`이므로 `await`를 사용해야 합니다
2. **네트워크 의존성**: 인터넷 연결이 필요합니다
3. **Rate Limiting**: API 호출 빈도를 적절히 조절하세요
4. **단위**: 모든 금액은 satoshi 단위입니다 (1 BTC = 100,000,000 sats)

## 테스트 주소

Mutinynet에서 테스트할 수 있는 주소:
- `tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx` (잔고 있음)
- `tb1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gdcccefvpysxf3q0sl5k7`

## 문의 및 지원

이슈가 있거나 기능 추가가 필요한 경우 GitHub Issue를 생성해주세요.