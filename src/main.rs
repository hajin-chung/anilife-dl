use anilife_rs::{api, http::create_http_client};
use inquire::{MultiSelect, Select, Text};

#[macro_use]
extern crate log;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let _log2 = log2::open(".log.tmp").start();

  let client = create_http_client();

  loop {
    let query = match Text::new("Search: ").prompt() {
      Ok(q) => q,
      Err(error) => match error {
        inquire::InquireError::OperationCanceled => break,
        inquire::InquireError::OperationInterrupted => break,
        _ => {
          debug!("{}", error);
          continue;
        }
      },
    };

    let (anime_list, search_url) = api::search(&client, &query).await?;
    info!("{:?}", anime_list);

    let prompt = format!("Animes ({})", anime_list.len()).to_string();
    let anime = match Select::new(&prompt, anime_list).prompt() {
      Ok(a) => a,
      Err(error) => match error {
        inquire::InquireError::OperationCanceled => break,
        inquire::InquireError::OperationInterrupted => break,
        _ => {
          debug!("{}", error);
          continue;
        }
      },
    };

    let (episode_list, episode_url) = api::get_episodes(&client, &anime.url, &search_url).await?;
    let prompt = format!("{} ({})", anime.title, episode_list.len()).to_string();
    let episodes = match MultiSelect::new(&prompt, episode_list).prompt() {
      Ok(e) => e,
      Err(error) => match error {
        inquire::InquireError::OperationCanceled => break,
        inquire::InquireError::OperationInterrupted => break,
        _ => {
          debug!("{}", error);
          continue;
        }
      },
    };

    for episode in episodes {
      let hls_url = api::get_episode_hls(&client, &episode.url, &episode_url).await?;

      let filename = format!("{}-{}-{}", anime.title, episode.num, episode.title);
      api::download_episode(&client, &hls_url, &filename).await?;
    }
  }

  Ok(())
}
