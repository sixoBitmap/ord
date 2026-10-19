use super::*;

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
pub(crate) struct SpendingPrevout {
  #[serde(rename = "spendingtxid")]
  pub(crate) spending_txid: Option<Txid>,
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
  fn get_raw_transaction_info_ext(&self, txid: Txid) -> Result<Option<RawTransaction>>;
  fn get_tx_out_ext(&self, outpoint: OutPoint) -> Result<Option<TxOutInfo>>;
  fn get_tx_spending_prevout_ext(&self, outpoint: OutPoint) -> Result<Option<Txid>>;
}

impl ClientExt for Client {
  fn get_raw_transaction_info_ext(&self, txid: Txid) -> Result<Option<RawTransaction>> {
    self
      .call("getrawtransaction", &[txid.to_string().into(), true.into()])
      .into_option()
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

  fn get_tx_spending_prevout_ext(&self, outpoint: OutPoint) -> Result<Option<Txid>> {
    Ok(
      self
        .call::<Vec<SpendingPrevout>>(
          "gettxspendingprevout",
          &[serde_json::json!([{ "txid": outpoint.txid, "vout": outpoint.vout }])],
        )?
        .into_iter()
        .next()
        .and_then(|prevout| prevout.spending_txid),
    )
  }
}
