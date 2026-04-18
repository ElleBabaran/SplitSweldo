const express = require('express');
const cors = require('cors');
const { exec } = require('child_process');
const path = require('path');
const fs = require('fs');
require('dotenv').config();

const app = express();
app.use(cors());
app.use(express.json());

const CONTRACT_ID = process.env.CONTRACT_ID || '';
const NETWORK = process.env.NETWORK || 'testnet';
const USDC_TOKEN = process.env.USDC_TOKEN || 'CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA';

function stellarInvoke(fnName, args, source) {
  return new Promise((resolve, reject) => {
    const argStr = Object.entries(args)
      .map(([k, v]) => `--${k} "${v}"`)
      .join(' ');

    const cmd = `stellar contract invoke \
      --id ${CONTRACT_ID} \
      --source ${source} \
      --network ${NETWORK} \
      -- ${fnName} ${argStr}`;

    exec(cmd, (err, stdout, stderr) => {
      if (err) return reject(stderr || err.message);
      resolve(stdout.trim());
    });
  });
}

function stellarRead(fnName, args = {}) {
  return new Promise((resolve, reject) => {
    const argStr = Object.entries(args)
      .map(([k, v]) => `--${k} "${v}"`)
      .join(' ');

    const cmd = `stellar contract invoke \
      --id ${CONTRACT_ID} \
      --source employer \
      --network ${NETWORK} \
      -- ${fnName} ${argStr}`;

    exec(cmd, (err, stdout, stderr) => {
      if (err) return reject(stderr || err.message);
      try {
        resolve(JSON.parse(stdout.trim()));
      } catch {
        resolve(stdout.trim());
      }
    });
  });
}

app.get('/api/status', async (req, res) => {
  try {
    const [funded, employer, worker] = await Promise.all([
      stellarRead('get_funded_amount'),
      stellarRead('get_employer'),
      stellarRead('get_worker'),
    ]);
    res.json({
      contract_id: CONTRACT_ID,
      network: NETWORK,
      funded_amount: funded,
      funded_usdc: (Number(funded) / 1e7).toFixed(2),
      employer,
      worker,
    });
  } catch (err) {
    res.status(500).json({ error: err.toString() });
  }
});

app.get('/api/split-rules', async (req, res) => {
  try {
    const rules = await stellarRead('get_split_rules');
    res.json({ rules });
  } catch (err) {
    res.status(500).json({ error: err.toString() });
  }
});

app.post('/api/initialize', async (req, res) => {
  const { employer, worker } = req.body;
  if (!employer || !worker)
    return res.status(400).json({ error: 'employer and worker addresses required' });
  try {
    await stellarInvoke('initialize', { employer, worker }, 'employer');
    res.json({ success: true, message: 'Contract initialized' });
  } catch (err) {
    res.status(500).json({ error: err.toString() });
  }
});

app.post('/api/set-split-rules', async (req, res) => {
  const { token, rules } = req.body;
  if (!rules || !Array.isArray(rules))
    return res.status(400).json({ error: 'rules array required' });

  const total = rules.reduce((sum, r) => sum + r.bps, 0);
  if (total !== 10000)
    return res.status(400).json({ error: `BPS must sum to 10000, got ${total}` });

  // Write rules to temp file and use --rules-file-path (avoids shell quoting issues)
  const tmpFile = path.join(__dirname, 'tmp_rules.json');
  const formattedRules = rules.map(r => ({ wallet: r.wallet, bps: r.bps }));

  try {
    fs.writeFileSync(tmpFile, JSON.stringify(formattedRules), 'utf8');

    const cmd = `stellar contract invoke \
      --id ${CONTRACT_ID} \
      --source worker \
      --network ${NETWORK} \
      -- set_split_rules \
      --token "${token || USDC_TOKEN}" \
      --rules-file-path "${tmpFile}"`;

    await new Promise((resolve, reject) => {
      exec(cmd, (err, stdout, stderr) => {
        try { fs.unlinkSync(tmpFile); } catch {}
        if (err) return reject(stderr || err.message);
        resolve(stdout);
      });
    });

    res.json({ success: true, message: 'Split rules saved on-chain' });
  } catch (err) {
    try { fs.unlinkSync(tmpFile); } catch {}
    res.status(500).json({ error: err.toString() });
  }
});

app.post('/api/fund-payroll', async (req, res) => {
  const { amount_usdc } = req.body;
  if (!amount_usdc || isNaN(amount_usdc))
    return res.status(400).json({ error: 'amount_usdc required' });

  const amount = Math.round(parseFloat(amount_usdc) * 1e7);

  try {
    await stellarInvoke('fund_payroll', { amount }, 'employer');
    res.json({
      success: true,
      message: `${amount_usdc} USDC locked in contract`,
      stroops: amount,
    });
  } catch (err) {
    res.status(500).json({ error: err.toString() });
  }
});

app.post('/api/release-payroll', async (req, res) => {
  try {
    await stellarInvoke('release_payroll', {}, 'employer');
    res.json({
      success: true,
      message: 'Payroll released — atomic fan-out complete',
    });
  } catch (err) {
    res.status(500).json({ error: err.toString() });
  }
});

app.get('/api/balance/:address', async (req, res) => {
  const { address } = req.params;
  try {
    const cmd = `stellar contract invoke \
      --id ${USDC_TOKEN} \
      --source employer \
      --network ${NETWORK} \
      -- balance --id "${address}"`;

    const balance = await new Promise((resolve, reject) => {
      exec(cmd, (err, stdout, stderr) => {
        if (err) return reject(stderr || err.message);
        resolve(stdout.trim());
      });
    });

    res.json({
      address,
      balance_stroops: balance,
      balance_usdc: (Number(balance) / 1e7).toFixed(2),
    });
  } catch (err) {
    res.status(500).json({ error: err.toString() });
  }
});

const PORT = process.env.PORT || 3001;
app.listen(PORT, () => {
  console.log(`SplitSweldo API running on http://localhost:${PORT}`);
  console.log(`Contract: ${CONTRACT_ID || '(not set — add to .env)'}`);
  console.log(`Network:  ${NETWORK}`);
});


