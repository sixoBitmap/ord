use super::*;

#[derive(Boilerplate)]
pub(crate) struct MempoolInscriptionsHtml {
  pub(crate) inscriptions: Vec<api::MempoolInscription>,
  pub(crate) next: Option<u32>,
  pub(crate) now: i64,
  pub(crate) prev: Option<u32>,
}

impl MempoolInscriptionsHtml {
  fn age(&self, inscription: &api::MempoolInscription) -> String {
    let seconds = (self.now - inscription.timestamp).max(0);

    if seconds < 60 {
      format!("{seconds}s")
    } else if seconds < 60 * 60 {
      format!("{}m", seconds / 60)
    } else if seconds < 48 * 60 * 60 {
      format!("{}h", seconds / (60 * 60))
    } else {
      format!("{}d", seconds / (24 * 60 * 60))
    }
  }
}

impl PageContent for MempoolInscriptionsHtml {
  fn title(&self) -> String {
    "Mempool Inscriptions".into()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn mempool_inscription(n: u32) -> api::MempoolInscription {
    api::MempoolInscription {
      content_encoding: None,
      content_length: Some(13),
      content_type: Some("text/plain".into()),
      delegate: None,
      fee: 1680,
      fee_rate: 10.0,
      id: inscription_id(n),
      metaprotocol: None,
      parents: Vec::new(),
      timestamp: 0,
    }
  }

  #[test]
  fn without_prev_and_next() {
    assert_regex_match!(
      MempoolInscriptionsHtml {
        inscriptions: vec![mempool_inscription(1)],
        next: None,
        now: 90,
        prev: None,
      },
      "
        <h1>Mempool Inscriptions</h1>
        <table class=mempool-inscriptions>
          <tr>
            <th>inscription</th>
            <th>content type</th>
            <th>size</th>
            <th>fee rate</th>
            <th>age</th>
          </tr>
          <tr>
            <td><a class=collapse href=/tx/1{64}>1{64}i1</a></td>
            <td>text/plain</td>
            <td>13</td>
            <td>10.0 sat/vB</td>
            <td>1m</td>
          </tr>
        </table>
        <div class=center>
        prev
        next
        </div>
      "
      .unindent()
    );
  }

  #[test]
  fn with_prev_and_next() {
    assert_regex_match!(
      MempoolInscriptionsHtml {
        inscriptions: vec![mempool_inscription(1), mempool_inscription(2)],
        next: Some(3),
        now: 0,
        prev: Some(1),
      },
      "
        <h1>Mempool Inscriptions</h1>
        <table class=mempool-inscriptions>
          .*
          <tr>
            <td><a class=collapse href=/tx/1{64}>1{64}i1</a></td>
          .*
          <tr>
            <td><a class=collapse href=/tx/2{64}>2{64}i2</a></td>
          .*
        </table>
        <div class=center>
        <a class=prev href=/mempool/inscriptions/1>prev</a>
        <a class=next href=/mempool/inscriptions/3>next</a>
        </div>
      "
      .unindent()
    );
  }

  #[test]
  fn content_type_is_escaped() {
    let mut inscription = mempool_inscription(1);
    inscription.content_type = Some("<script>".into());

    assert_regex_match!(
      MempoolInscriptionsHtml {
        inscriptions: vec![inscription],
        next: None,
        now: 0,
        prev: None,
      },
      ".*<td>&lt;script&gt;</td>.*"
    );
  }

  #[test]
  fn age() {
    let mut page = MempoolInscriptionsHtml {
      inscriptions: Vec::new(),
      next: None,
      now: 0,
      prev: None,
    };

    let inscription = mempool_inscription(1);

    for (now, age) in [
      (-5, "0s"),
      (59, "59s"),
      (60, "1m"),
      (3599, "59m"),
      (3600, "1h"),
      (47 * 3600, "47h"),
      (48 * 3600, "2d"),
    ] {
      page.now = now;
      assert_eq!(page.age(&inscription), age);
    }
  }
}
