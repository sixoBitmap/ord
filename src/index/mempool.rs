use {
  super::*,
  std::{cmp::Reverse, sync::RwLock},
};

const BATCH_SIZE: usize = 1000;
const MAX_FIELD_LENGTH: usize = 128;

type Key = (Reverse<i64>, InscriptionId);

#[derive(Default)]
pub(crate) struct Mempool {
  state: RwLock<State>,
  update: Mutex<()>,
}

#[derive(Default)]
struct State {
  inscriptions: BTreeMap<Key, api::MempoolInscription>,
  transactions: HashMap<Txid, Vec<Key>>,
}

impl Mempool {
  pub(crate) fn update(&self, client: &Client) -> Result {
    let _update = self.update.lock().unwrap();

    let txids = client
      .get_raw_mempool()?
      .into_iter()
      .collect::<HashSet<Txid>>();

    let (added, removed) = {
      let state = self.state.read().unwrap();

      (
        txids
          .iter()
          .filter(|txid| !state.transactions.contains_key(*txid))
          .copied()
          .collect::<Vec<Txid>>(),
        state
          .transactions
          .keys()
          .filter(|txid| !txids.contains(*txid))
          .copied()
          .collect::<Vec<Txid>>(),
      )
    };

    let added = Self::fetch(client, &added)?;

    self.apply(added, &removed);

    Ok(())
  }

  fn fetch(client: &Client, txids: &[Txid]) -> Result<Vec<(Txid, Vec<api::MempoolInscription>)>> {
    let mut transactions = Vec::new();

    for chunk in txids.chunks(BATCH_SIZE) {
      for (txid, transaction) in chunk.iter().zip(client.get_raw_transactions_ext(chunk)?) {
        if let Some(transaction) = transaction {
          transactions.push((*txid, ParsedEnvelope::from_transaction(&transaction)));
        }
      }
    }

    let inscribed = transactions
      .iter()
      .filter(|(_, envelopes)| !envelopes.is_empty())
      .map(|(txid, _)| *txid)
      .collect::<Vec<Txid>>();

    let mut entries = HashMap::new();

    for chunk in inscribed.chunks(BATCH_SIZE) {
      for (txid, entry) in chunk.iter().zip(client.get_mempool_entries_ext(chunk)?) {
        if let Some(entry) = entry {
          entries.insert(*txid, entry);
        }
      }
    }

    Ok(
      transactions
        .into_iter()
        .filter_map(|(txid, envelopes)| {
          if envelopes.is_empty() {
            return Some((txid, Vec::new()));
          }

          let entry = entries.get(&txid)?;

          let count = u64::try_from(envelopes.len()).unwrap();

          Some((
            txid,
            envelopes
              .into_iter()
              .enumerate()
              .map(|(index, envelope)| {
                let inscription = envelope.payload;

                api::MempoolInscription {
                  content_encoding: inscription
                    .content_encoding()
                    .and_then(|encoding| encoding.to_str().ok().map(truncate)),
                  content_length: inscription.content_length(),
                  content_type: inscription.content_type().map(truncate),
                  delegate: inscription.delegate(),
                  fee: entry.fees.base.to_sat() / count,
                  fee_rate: entry.fees.base.to_sat() as f64 / entry.vsize.max(1) as f64,
                  id: InscriptionId {
                    txid,
                    index: u32::try_from(index).unwrap(),
                  },
                  metaprotocol: inscription.metaprotocol().map(truncate),
                  parents: inscription.parents().into_iter().take(4).collect(),
                  timestamp: entry.time,
                }
              })
              .collect(),
          ))
        })
        .collect(),
    )
  }

  fn apply(&self, added: Vec<(Txid, Vec<api::MempoolInscription>)>, removed: &[Txid]) {
    let mut state = self.state.write().unwrap();

    for txid in removed {
      for key in state.transactions.remove(txid).unwrap_or_default() {
        state.inscriptions.remove(&key);
      }
    }

    for (txid, inscriptions) in added {
      let mut keys = Vec::new();

      for inscription in inscriptions {
        let key = (Reverse(inscription.timestamp), inscription.id);
        state.inscriptions.insert(key, inscription);
        keys.push(key);
      }

      state.transactions.insert(txid, keys);
    }
  }

  pub(crate) fn inscriptions_paginated(
    &self,
    page_size: usize,
    page_index: usize,
  ) -> (Vec<api::MempoolInscription>, bool) {
    let state = self.state.read().unwrap();

    let mut inscriptions = state
      .inscriptions
      .values()
      .skip(page_index.saturating_mul(page_size))
      .take(page_size.saturating_add(1))
      .cloned()
      .collect::<Vec<api::MempoolInscription>>();

    let more = inscriptions.len() > page_size;

    if more {
      inscriptions.pop();
    }

    (inscriptions, more)
  }
}

fn truncate(value: &str) -> String {
  value.chars().take(MAX_FIELD_LENGTH).collect()
}
