use super::*;

#[derive(Deserialize)]
pub(crate) struct MempoolEntry {
  pub(crate) fees: MempoolEntryFees,
  pub(crate) time: i64,
  pub(crate) vsize: u64,
}

#[derive(Deserialize)]
pub(crate) struct MempoolEntryFees {
  #[serde(with = "bitcoin::amount::serde::as_btc")]
  pub(crate) base: Amount,
}

#[derive(Deserialize)]
pub(crate) struct RawTransaction {
  pub(crate) blockhash: Option<BlockHash>,
  pub(crate) confirmations: Option<u32>,
  #[serde(with = "consensus::serde::With::<consensus::serde::Hex>")]
  pub(crate) hex: Transaction,
}

#[derive(Deserialize)]
pub(crate) struct ScriptPubKey {
  pub(crate) hex: ScriptBuf,
}

#[derive(Deserialize)]
pub(crate) struct TxOutInfo {
  pub(crate) confirmations: u32,
  #[serde(rename = "scriptPubKey")]
  pub(crate) script_pub_key: ScriptPubKey,
  #[serde(with = "bitcoin::amount::serde::as_btc")]
  pub(crate) value: Amount,
}

pub(crate) trait ClientExt {
  fn get_mempool_entries_ext(&self, txids: &[Txid]) -> Result<Vec<Option<MempoolEntry>>>;
  fn get_raw_transaction_info_ext(&self, txid: Txid) -> Result<Option<RawTransaction>>;
  fn get_raw_transactions_ext(&self, txids: &[Txid]) -> Result<Vec<Option<Transaction>>>;
  fn get_tx_out_ext(&self, outpoint: OutPoint) -> Result<Option<TxOutInfo>>;
}

impl ClientExt for Client {
  fn get_mempool_entries_ext(&self, txids: &[Txid]) -> Result<Vec<Option<MempoolEntry>>> {
    call_batch(self, "getmempoolentry", txids)
  }

  fn get_raw_transaction_info_ext(&self, txid: Txid) -> Result<Option<RawTransaction>> {
    self
      .call("getrawtransaction", &[txid.to_string().into(), true.into()])
      .into_option()
  }

  fn get_raw_transactions_ext(&self, txids: &[Txid]) -> Result<Vec<Option<Transaction>>> {
    call_batch::<String>(self, "getrawtransaction", txids)?
      .into_iter()
      .map(|hex| {
        hex
          .map(|hex| consensus::encode::deserialize_hex(&hex))
          .transpose()
          .map_err(Into::into)
      })
      .collect()
  }

  fn get_tx_out_ext(&self, outpoint: OutPoint) -> Result<Option<TxOutInfo>> {
    Ok(self.call(
      "gettxout",
      &[
        outpoint.txid.to_string().into(),
        outpoint.vout.into(),
        true.into(),
      ],
    )?)
  }
}

fn call_batch<T: serde::de::DeserializeOwned>(
  client: &Client,
  method: &str,
  txids: &[Txid],
) -> Result<Vec<Option<T>>> {
  if txids.is_empty() {
    return Ok(Vec::new());
  }

  let jsonrpc = client.get_jsonrpc_client();

  let params = txids
    .iter()
    .map(|txid| serde_json::value::to_raw_value(&[txid]))
    .collect::<Result<Vec<_>, _>>()?;

  let requests = params
    .iter()
    .map(|params| jsonrpc.build_request(method, Some(params)))
    .collect::<Vec<_>>();

  Ok(
    jsonrpc
      .send_batch(&requests)?
      .into_iter()
      .map(|response| response.and_then(|response| response.result().ok()))
      .collect(),
  )
}
