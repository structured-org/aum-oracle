# AUM Oracle Operator Guide v2

## Overview

The AUM Oracle is a decentralized oracle system that aggregates Assets Under Management (AUM) data from multiple sources, including Binance and Jupiter/Solana. The system employs a Byzantine fault-tolerant consensus mechanism to ensure data reliability and accuracy.

### System Architecture

The AUM Oracle consists of two main components:

1. **AUM Messenger**: Collects and submits AUM data from external sources (Binance and Jupiter/Solana) to on-chain contracts
2. **Recorder**: Periodically executes the `record_er` function in the TWAER (Time-Weighted Average Exchange Rate) contract to create exchange rate records

Both services work together to provide comprehensive AUM tracking and time-weighted exchange rate calculations.

### How It Works

1. **Multiple oracle nodes** (operated by different participants) independently fetch data from external sources
2. **AUM Messenger service** submits AUM data to on-chain smart contracts on Neutron
3. **Recorder service** periodically creates TWAER records by executing the `record_er` function in the TWAER contract
4. **Consensus mechanism** validates and aggregates the data (requires 2/3 agreement by default)
5. **Final AUM values** are calculated and made available on-chain

### Current Decentralization Plan

The oracle network is being progressively decentralized in phases:

**Phase 1 (Current):** 3 nodes, all operated internally, 2/3 consensus threshold
**Phase 2:** 5 nodes total - 3 internal + 2 external operators, 3/5 consensus threshold
**Phase 3:** 5 nodes total - 1 internal + 4 external operators, maintaining 3/5 threshold

---

## System Requirements

### Minimum Specifications

- **CPU:** 1 vCPU (x86_64), Intel Skylake/AMD EPYC or newer
- **RAM:** 1 GB
- **Storage:** 10 GB SSD
    - IOPS: 100 minimum
    - Throughput: 125 MB/s
- **Network:** 1 Mbps sustained, 10 Mbps burst, <100ms latency to endpoints

### Recommended Specifications for Production

### Cloud Instances

| Provider | Instance Type | Specs | Notes |
| --- | --- | --- | --- |
| **AWS** | t3.large | 2 vCPU, 8 GB RAM | Intel Xeon Platinum 8000 |
| **GCP** | e2-medium | 2 vCPU, 4 GB RAM | Cost-optimized |
| **Azure** | B2s | 2 vCPU, 4 GB RAM | Burstable |
| **DigitalOcean** | Basic 2GB | 1 vCPU, 2 GB RAM | Budget option |

### Storage

- **Type:** General Purpose SSD (gp3/gp2)
- **Size:** 20 GB
- **IOPS:** 3,000 (gp3) or 100 baseline (gp2)
- **Throughput:** 125 MB/s

### Network

- **Bandwidth:** 5 Mbps peak
- **Latency:** <50ms to Neutron, <100ms to Solana/Binance
- **Monthly transfer:** 5-10 GB

### OS / Container Engine

- **Operating System:**
    - Ubuntu 24.04 LTS
    - Rocky Linux 9
- **Container Engine (choose one):**
    - **Docker Engine:** v24.0 or newer (v28.x recommended)
    - **Docker Compose:** v2.20 or newer (installed as plugin)
    - **Podman:** v4.9+
    - **Podman-compose:** Required if using Podman

### Network Requirements

The oracle node requires outbound HTTPS access to:

1. **Neutron RPC** (required)
    - Production: `https://rpc-lb.neutron.org`
    - Port: 443 (HTTPS)
2. **Solana RPC** (required)
    - Default: `https://api.mainnet-beta.solana.com`
    - Alternative providers can be used (Helius, QuickNode, etc.)
    - Port: 443 (HTTPS)
3. **Binance API** (required)
    - API endpoint: `https://api.binance.com`
    - Port: 443 (HTTPS)

**Firewall Configuration:** Only outbound HTTPS (port 443) is required. No inbound ports need to be exposed.

---

## External API Requirements

### 1. Binance API

Binance API keys will be provided by Structured during onboarding. These keys give read-only access to Structured's Binance account. Simply add the provided credentials to your config file.

**What You Need:**

- Binance API keys will be provided to you by the Structured team during the onboarding process
- These keys provide read-only access to the Structured Binance account
- Simply add the provided API key and secret to your configuration file

**API Key Details:**

- ✅ Read-only access to account data
- ❌ No trading permissions
- ❌ No withdrawal permissions

**Security Note:**
Keep the provided API credentials secure and do not share them with anyone outside your organization.

### 2. Solana RPC Endpoint

The default public endpoint (`https://api.mainnet-beta.solana.com`) has rate limits. For production, consider using:

### Recommended RPC Providers:

- **Helius:** [https://helius.dev](https://helius.dev/)
- **QuickNode:** [https://quicknode.com](https://quicknode.com/)
- **Alchemy:** [https://alchemy.com](https://alchemy.com/)
- **Triton:** [https://triton.one](https://triton.one/)

### Setup:

1. Sign up with your chosen provider
2. Create a Solana Mainnet endpoint
3. Copy the HTTPS URL
4. Replace the default endpoint in your configuration

### 3. Neutron Chain Access

No special setup required. The oracle uses public RPC endpoints:

- **Mainnet:** `https://rpc-lb.neutron.org`
- **Chain ID:** `neutron-1`

---

## Installation & Deployment

### Prerequisites

Choose either Docker or Podman for container management:

```bash
# 1. Update system packages
sudo apt update && sudo apt upgrade -y

# 2. Install prerequisites
sudo apt install ca-certificates curl gnupg lsb-release

# 3. Add Docker's official GPG key and repository
sudo mkdir -p /etc/apt/keyrings
curl -fsSL <https://download.docker.com/linux/ubuntu/gpg> | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg
echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] <https://download.docker.com/linux/ubuntu> $(lsb_release -cs) stable" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null

# 4. Install Docker Engine and Docker Compose plugin
sudo apt install docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

# 5. Add your user to docker group (optional, for non-root operation)
sudo usermod -aG docker $USER

# 6. Verify installation
docker --version
docker compose version  # Note: "docker compose" not "docker-compose"

# 7. Enable Docker to start on boot
sudo systemctl enable docker

# Log out and back in for group changes to take effect
```

### Step 1: Clone the Repository

```bash
# Clone the open-source repository
git clone <https://github.com/structured-org/aum-oracle.git>
cd aum-oracle
```

### Step 2: Configure the AUM Messenger

Create your AUM Messenger configuration file from the template:

```bash
# Create config file from template
cp aum_messenger/config.yaml.default aum_messenger/config.yaml
```

Edit `aum_messenger/config.yaml` with your specific values:

```yaml
# Asset Lists - These should match exactly across all operators
binance_um_positions_list:
  - BTCUSDT
  - ETHUSDT
  - SOLUSDT
  - BNBUSDT

binance_spot_assets_list:
  - USDT
  - BTC
  - ETH
  - SOL
  - BNB
  - WBTC
  - USDC

# Jupiter Configuration - DO NOT MODIFY (must be identical for all nodes)
jupiter_custodies:
  SOL: "7xS2gz2bTp3fwCC7knJvUWTEU9Tycczu6VhJYKgi1wdz"
  WETH: "AQCGyheWPLeo6Qp9WpYS9m3Qj479t7R636N9ey1rEjEn"
  WBTC: "5Pv3gM9JrFFH883SWAhvJC9RPYmo8UNxuFtv5bMMALkm"
  USDC: "G18jKKXQwBbrHeiK3C9MRXhkHsLHf7XgCSisykV46EZa"
  USDT: "4vkNeXiYEUizLdrpdPS1eC2mccyM4NUPRtERrk6ZETkk"
jupiter_jlp_token: "27G8MtK7VtTcCHkpASjSDdkWWYfoqT6ggEuKidVJidD4"
jupiter_jlp_pool: "5BUwFW4nRbftYTDMbgxykoFWqWHPzahFSNAaaaJtVKsq"
jupiter_strategy_address: "AYBzGpmCGLvLJQHAKEFZGBAm3R8yER7K4c3GZ2YgMEnf"

# Contract Addresses - Provided by the oracle network administrator
jupiter_aum_contract: "neutron1s048a22wjsk3jgzdft5fhsdkdted45ne9hzfwxvg38rxnlq7lutq86fppg"
binance_aum_contract: "neutron15mxl4juekpr8dk636rxkqvw4efx873sfs59ve97lkl0f02up4vhqr5j8kz"

# Operational Configuration - Recommended defaults
operational_config:
  failure_delay: 1s          # Delay after a failed submission
  pre_submit_delay: 3s       # Delay before submitting to avoid conflicts
  fetch_data_timeout: 3s     # Timeout for external API calls

# Client Configuration
clients:
  # Neutron Configuration - YOUR UNIQUE WALLET
  neutron:
    mnemonic: "YOUR 24 WORD MNEMONIC HERE"  # IMPORTANT: Keep this secret!
    gas_prices: "0.0053untrn"
    gas_adjustment: 1.5
    chain_id: "neutron-1"
    node: "<https://rpc-lb.neutron.org>"
    node_conn_retries: 5
    node_conn_retry_delay: "3s"

  # Solana Configuration
  solana:
    rpc_endpoint: "<https://api.mainnet-beta.solana.com>"  # Or your provider URL

  # Binance Configuration - PROVIDED BY STRUCTURED
  binance:
    api_key: "YOUR_BINANCE_API_KEY"
    api_secret: "YOUR_BINANCE_API_SECRET"

# System Configuration
mock_clients: false          # Must be false for production
mock_controller_port: 3000   # Not used in production
logger_level: info           # Options: debug, info, warn, error
```

### Step 3: Configure the Recorder

Create your Recorder configuration file from the template:

```bash
# Create recorder config file from template
cp recorder/config.yaml.default recorder/config.yaml
```

Edit `recorder/config.yaml` with your specific values:

```yaml
# TWAER contract address where record_er will be executed
# This address will be provided by the oracle network administrator
twaer_contract: "YOUR_TWAER_CONTRACT_ADDRESS"

# Interval between record_er executions (e.g., "30s", "1m", "5m")
# Default is typically 30 seconds
record_interval: 30s

# Client Configuration
clients:
  neutron:
    mnemonic: "YOUR 24 WORD MNEMONIC HERE"  # Can use the same wallet as AUM Messenger
    gas_prices: "0.0053untrn"
    gas_adjustment: 1.5
    chain_id: "neutron-1"
    node: "<https://rpc-lb.neutron.org>"
    node_conn_retries: 5
    node_conn_retry_delay: "3s"

# System Configuration
logger_level: info           # Options: debug, info, warn, error
```

**Important Note:** Both the AUM Messenger and Recorder can use the same Neutron wallet (same mnemonic). This simplifies wallet management and reduces operational overhead.

### Step 4: Wallet Setup

Create a Neutron wallet for your oracle node:

### Option A: Generate New Wallet

```bash
# Install neutrond CLI (if not already installed)
wget <https://github.com/neutron-org/neutron/releases/latest/download/neutrond-linux-amd64>
chmod +x neutrond-linux-amd64
sudo mv neutrond-linux-amd64 /usr/local/bin/neutrond

# Generate new wallet
neutrond keys add oracle-operator --keyring-backend aum-oracle

# IMPORTANT: Save the 24-word mnemonic securely!
# You'll need it for both config files
```

### Option B: Import Existing Wallet

```bash
neutrond keys add oracle-operator --recover --keyring-backend aum-oracle
# Enter your 24-word mnemonic when prompted
```

### Fund Your Wallet

Your wallet needs NTRN tokens for gas fees:

- **Minimum:** 20 NTRN
- **Recommended:** 150 NTRN for several months of operation
- **Get your address:** `neutrond keys show oracle-operator -a --keyring-backend aum-oracle`
- Request funding from the oracle network administrator

### Step 5: Build and Run Both Services

The oracle system uses a root-level `docker-compose.yml` file that manages both services:

```bash
# From the repository root directory
# Ensure both config files are in place:
# - aum_messenger/config.yaml
# - recorder/config.yaml

# Build and start both services (first time)
docker compose up -d --build

# For subsequent starts (no rebuild needed)
docker compose up -d

# Start only specific service if needed
docker compose up -d aum-messenger  # Only AUM Messenger
docker compose up -d recorder       # Only Recorder

# Check logs for both services
docker compose logs -f

# Check logs for specific service
docker compose logs -f aum-messenger
docker compose logs -f recorder

# Stop both services
docker compose down

# Stop specific service
docker compose stop aum-messenger
docker compose stop recorder
```

### Step 6: Verify Operation

Check that both services are running correctly:

```bash
# View logs for both services
docker compose logs -f

# View logs for AUM Messenger only
docker compose logs -f aum-messenger
# Look for successful submissions:
# "Successfully submitted Binance data"
# "Successfully submitted Jupiter data"

# View logs for Recorder only
docker compose logs -f recorder
# Look for successful executions:
# "Executing record_er"
# "Successfully executed record_er"

# Check container status (should see both containers)
docker ps

# Monitor resource usage for both services
docker stats aum-messenger recorder
```

Expected log patterns:

**AUM Messenger:**

- Data fetching every ~30 seconds
- Successful submissions to both contracts
- No repeated error messages

**Recorder:**

- Periodic execution of record_er based on configured interval
- Successful record_er executions
- No repeated error messages

---

## Monitoring & Maintenance

### Health Monitoring

Set up monitoring to ensure both services stay healthy:

### Enhanced Health Check Script

```bash
#!/bin/bash
# save as check_oracle_health.sh

# Check if both containers are running
if ! docker ps | grep -q aum-messenger; then
  echo "ERROR: AUM Messenger container is not running"
  exit 1
fi

if ! docker ps | grep -q recorder; then
  echo "ERROR: Recorder container is not running"
  exit 1
fi

# Check AUM Messenger for recent errors
if docker logs aum-messenger --since 5m 2>&1 | grep -q "ERROR"; then
  echo "WARNING: Recent errors found in AUM Messenger logs"
  docker logs aum-messenger --since 5m 2>&1 | grep "ERROR"
fi

# Check Recorder for recent errors
if docker logs recorder --since 5m 2>&1 | grep -q "ERROR"; then
  echo "WARNING: Recent errors found in Recorder logs"
  docker logs recorder --since 5m 2>&1 | grep "ERROR"
fi

# Check for successful AUM Messenger submissions
if ! docker logs aum-messenger --since 10m 2>&1 | grep -q "Successfully submitted"; then
  echo "WARNING: No successful AUM submissions in last 10 minutes"
fi

# Check for successful Recorder executions
if ! docker logs recorder --since 10m 2>&1 | grep -q "Successfully executed record_er"; then
  echo "WARNING: No successful record_er executions in last 10 minutes"
fi

echo "Both oracle services appear healthy"
```

### Recommended Monitoring Solutions

- **Prometheus + Grafana** for metrics visualization
- **Uptime Kuma** for simple uptime monitoring
- **CloudWatch** (AWS) or equivalent cloud monitoring

### Log Management

```bash
# View live logs for both services
docker compose logs -f

# View live logs for specific service
docker compose logs -f aum-messenger
docker compose logs -f recorder

# View last 100 lines from both services
docker compose logs --tail 100

# View last 100 lines from specific service
docker compose logs --tail 100 aum-messenger

# Save logs to file
docker compose logs aum-messenger > aum_messenger_logs_$(date +%Y%m%d).txt
docker compose logs recorder > recorder_logs_$(date +%Y%m%d).txt

# Set up log rotation
cat > /etc/logrotate.d/docker-aum-oracle <<EOF
/var/lib/docker/containers/*/*.log {
  rotate 7
  daily
  compress
  missingok
  notifempty
}
EOF
```

### Troubleshooting Common Issues

### Issue: "Failed to submit data" errors (AUM Messenger)

**Causes & Solutions:**

- **Insufficient gas:** Fund your wallet with more NTRN
- **RPC endpoint down:** Try alternative Neutron RPC endpoints
- **Network connectivity:** Check firewall and DNS settings

### Issue: "Context deadline exceeded"

**Causes & Solutions:**

- **Slow RPC response:** Increase `fetch_data_timeout` in config
- **Network latency:** Use geographically closer RPC endpoints

### Issue: "Unauthorized" from Binance

**Causes & Solutions:**

- **Invalid API credentials:** Request proper API keys from Structured
- **Expired keys:** Contact Structured for new API keys if they have expired

**Note:** If you experience persistent authentication issues, contact Structured support immediately.

### Issue: Container keeps restarting

**Causes & Solutions:**

- **Invalid config:** Validate YAML syntax in both config files
- **Missing required fields:** Check all config fields are set correctly
- **Permission issues:** Ensure config files are readable (`chmod 644`)

### Issue: Recorder not executing

**Causes & Solutions:**

- **Wrong TWAER contract address:** Verify the `twaer_contract` address in `recorder/config.yaml`
- **Insufficient gas:** Ensure wallet has sufficient NTRN tokens
- **Record interval too short:** If executions are failing due to timing, increase `record_interval`
- **Network connectivity:** Check connection to Neutron RPC endpoint

---

## Node Upgrades

### Coordinated Upgrade Process

Oracle upgrades must be coordinated across all operators to maintain consensus. The process follows these steps:

### 1. Notification Phase

- Network administrator announces upgrade via official channels
- Upgrade window is scheduled (typically 48-72 hours notice)
- Release notes and testing instructions provided

### 2. Preparation Phase

```bash
# Pull latest code changes
cd aum-oracle
git fetch origin
git checkout <new-version-tag>

# Review changes
git diff HEAD~1 HEAD

# Build new images (don't deploy yet)
docker compose build
```

### 3. Coordinated Deployment

At the scheduled time:

```bash
# Stop both services
docker compose down

# Backup current configs
cp aum_messenger/config.yaml aum_messenger/config.yaml.backup
cp recorder/config.yaml recorder/config.yaml.backup

# Apply any config changes if required
# (instructions will be provided for each upgrade)

# Start with new version
docker compose up -d

# Verify operation of both services
docker compose logs -f
```

### 4. Rollback Procedure (if needed)

```bash
# Stop problematic version
docker compose down

# Restore previous configs
cp aum_messenger/config.yaml.backup aum_messenger/config.yaml
cp recorder/config.yaml.backup recorder/config.yaml

# Checkout previous version
git checkout <previous-version-tag>

# Rebuild and restart
docker compose build
docker compose up -d
```

### Upgrade Communication Channels

Official communication channels for upgrades:

- **Primary:** Discord/Telegram oracle-operators channel
- **Backup:** Email list (register with administrator)
- **Emergency:** On-chain governance proposals

### Best Practices for Upgrades

1. **Always test in a staging environment first** if possible
2. **Keep backups** of working configurations
3. **Monitor closely** after upgrades for 30 minutes
4. **Report issues immediately** to the administrator
5. **Never skip versions** unless explicitly instructed

---

## Security Best Practices

### 1. Secure Your Configuration

```bash
# Set restrictive permissions
chmod 600 aum_messenger/config.yaml
chmod 600 recorder/config.yaml

# Never commit config files to git
echo "aum_messenger/config.yaml" >> .gitignore
echo "recorder/config.yaml" >> .gitignore

# Use environment variables for sensitive data (optional)
export NEUTRON_MNEMONIC="your mnemonic here"
export BINANCE_API_KEY="your api key"
export BINANCE_API_SECRET="your api secret"
```

### 2. Secure Your Server

```bash
# Enable firewall (Ubuntu/Debian)
sudo ufw enable
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow ssh

# Keep system updated
sudo apt update && sudo apt upgrade -y

# Enable automatic security updates
sudo apt install unattended-upgrades
sudo dpkg-reconfigure -plow unattended-upgrades
```

### 3. Wallet Security

- **Never share your mnemonic** with anyone
- **Use a dedicated wallet** for oracle operations only
- **Keep minimal funds** (enough for ~1 month of operations)
- **Monitor for unauthorized transactions**

### 4. API Key Management

- **Use read-only permissions** for all API keys
- **Rotate keys regularly** (every 3-6 months)
- **Monitor API usage** for anomalies
- **Contact Structured** for any API key issues

### 5. Backup Strategy

```bash
# Backup script
#!/bin/bash
BACKUP_DIR="/backup/aum-oracle"
DATE=$(date +%Y%m%d-%H%M%S)

mkdir -p $BACKUP_DIR

# Backup configs (encrypted)
tar czf - aum_messenger/config.yaml recorder/config.yaml | gpg --symmetric --cipher-algo aes256 > $BACKUP_DIR/configs-$DATE.tar.gz.gpg

# Backup logs
docker compose logs aum-messenger > $BACKUP_DIR/aum-messenger-logs-$DATE.txt
docker compose logs recorder > $BACKUP_DIR/recorder-logs-$DATE.txt

# Keep only last 30 days of backups
find $BACKUP_DIR -type f -mtime +30 -delete
```

---

## Operational Costs

### Estimated Monthly Costs

### Infrastructure

- **Cloud Server (t3.large):** ~$60-80/month
- **Storage:** ~$5/month
- **Data Transfer:** ~$5/month
- **Total Infrastructure:** ~$70-90/month

### Blockchain Fees

- **Gas Fees (NTRN):** ~15-30 NTRN/month (includes both AUM Messenger and Recorder)

### API Costs

- **Solana RPC:** Paid tiers from $49/month
- **Binance API:** Free (provided by Structured)
- **Total API:** $49/month

**Total Monthly Operating Cost:** $120-160/month

---

## Support & Resources

### Getting Help

1. **Documentation:**
    - GitHub Repository: [https://github.com/structured-org/aum-oracle](https://github.com/structured-org/aum-oracle)
    - This guide (kept updated with releases)
2. **Community Support:**
    - Discord: [Oracle Operators Channel]
    - Telegram: [Oracle Operators Group]
3. **Technical Issues:**
    - GitHub Issues: [https://github.com/structured-org/aum-oracle/issues](https://github.com/structured-org/aum-oracle/issues)
    - Include: Error logs, config (without secrets), steps to reproduce
4. **Emergency Contacts:**
    - Network Administrator: [Contact provided during onboarding]
    - On-call Support: [Provided for critical issues]

### Useful Commands Reference

```bash
# Container Management (Both Services)
docker compose up -d                  # Start both services
docker compose down                   # Stop both services
docker compose restart                # Restart both services
docker compose logs -f                # View live logs from both
docker ps                             # Check container status

# Individual Service Management
docker compose up -d aum-messenger    # Start only AUM Messenger
docker compose up -d recorder         # Start only Recorder
docker compose restart aum-messenger  # Restart AUM Messenger
docker compose restart recorder       # Restart Recorder
docker compose logs -f aum-messenger  # View AUM Messenger logs
docker compose logs -f recorder       # View Recorder logs

# Debugging
docker compose exec aum-messenger sh  # Enter AUM Messenger container
docker compose exec recorder sh       # Enter Recorder container
docker stats aum-messenger recorder   # Monitor resource usage
docker compose logs --tail 1000 aum-messenger | grep ERROR  # Find AUM Messenger errors
docker compose logs --tail 1000 recorder | grep ERROR       # Find Recorder errors

# Maintenance
docker system prune -a                # Clean unused Docker resources
docker compose pull                   # Update to latest images
docker compose build --no-cache       # Rebuild from scratch
```

---

## Appendix A: Configuration Reference

### AUM Messenger Configuration Schema

```yaml
# Required: Asset tracking lists
binance_um_positions_list: [string array]  # Futures positions to track
binance_spot_assets_list: [string array]   # Spot assets to track

# Required: Jupiter protocol addresses (DO NOT MODIFY)
jupiter_custodies: {map}          # Token custody addresses
jupiter_jlp_token: string         # JLP token address
jupiter_jlp_pool: string          # JLP pool address
jupiter_strategy_address: string  # Strategy contract address

# Required: Oracle receiver contracts
jupiter_aum_contract: string      # Neutron contract for Jupiter data
binance_aum_contract: string      # Neutron contract for Binance data

# Required: Operational parameters
operational_config:
  failure_delay: duration         # Delay after failed submission (e.g., "1s")
  pre_submit_delay: duration      # Delay before submission (e.g., "3s")
  fetch_data_timeout: duration    # External API timeout (e.g., "3s")

# Required: Client configurations
clients:
  neutron:
    mnemonic: string              # 24-word wallet mnemonic
    gas_prices: string            # Gas price (e.g., "0.0053untrn")
    gas_adjustment: float         # Gas multiplier (e.g., 1.5)
    chain_id: string              # Chain ID ("neutron-1" for mainnet)
    node: string                  # RPC endpoint URL
    node_conn_retries: int        # Connection retry attempts
    node_conn_retry_delay: duration # Delay between retries

  solana:
    rpc_endpoint: string          # Solana RPC URL

  binance:
    api_key: string               # Binance API key (provided by Structured)
    api_secret: string            # Binance API secret (provided by Structured)

# Required: System settings
mock_clients: boolean             # Must be false for production
mock_controller_port: int         # Port for mock controller (unused in production)
logger_level: string              # Log verbosity: debug|info|warn|error
```

### Recorder Configuration Schema

```yaml
# Required: TWAER contract configuration
twaer_contract: string            # TWAER contract address (provided by administrator)

# Required: Execution interval
record_interval: duration         # Time between record_er executions (e.g., "30s", "1m", "5m")

# Required: Client configuration
clients:
  neutron:
    mnemonic: string              # 24-word wallet mnemonic (can be same as AUM Messenger)
    gas_prices: string            # Gas price (e.g., "0.0053untrn")
    gas_adjustment: float         # Gas multiplier (e.g., 1.5)
    chain_id: string              # Chain ID ("neutron-1" for mainnet)
    node: string                  # RPC endpoint URL
    node_conn_retries: int        # Connection retry attempts
    node_conn_retry_delay: duration # Delay between retries

# Required: System settings
logger_level: string              # Log verbosity: debug|info|warn|error
```

**Important Notes:**

- Both services can use the same Neutron wallet (same mnemonic)
- The `twaer_contract` address will be provided by the oracle network administrator
- The `record_interval` should typically be set to 30 seconds or as instructed by the administrator

---

## Appendix B: Network Information

### Mainnet Configuration

```yaml
# Neutron Mainnet
chain_id: "neutron-1"
rpc_endpoints:
  - <https://rpc-lb.neutron.org>
  - <https://neutron-rpc.polkachu.com>
  - <https://rpc-neutron.whispernode.com>
gas_prices: "0.0053untrn"

# Solana Mainnet
rpc_endpoints:
  - <https://api.mainnet-beta.solana.com>
  - <https://solana-api.projectserum.com>
```

### Contract Addresses (Current)

```yaml
# These addresses may change - always use the latest provided by administrator
jupiter_aum_contract: "neutron1s048a22wjsk3jgzdft5fhsdkdted45ne9hzfwxvg38rxnlq7lutq86fppg"
binance_aum_contract: "neutron15mxl4juekpr8dk636rxkqvw4efx873sfs59ve97lkl0f02up4vhqr5j8kz"
twaer_contract: "YOUR_TWAER_CONTRACT_ADDRESS"  # Provided during onboarding
```

---

*This guide is maintained by the Structured team. Last updated: November 2025
*For the latest version, check:* [OPERATOR_GUIDE.md](https://github.com/structured-org/aum-oracle/docs/OPERATOR_GUIDE.md)

References:

[AUM oracle system](https://www.notion.so/AUM-oracle-system-1f285d6b9b1080ffbcfad24166bbeb0d?pvs=21) 

https://github.com/structured-org/aum-oracle