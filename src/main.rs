use anilife_rs::{api::LifeClient, dl::Downloader};
use inquire::{Select, Text};

#[macro_use]
extern crate log;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let _log2 = log2::open(".log.tmp").start();

  let client = LifeClient::new();
  let downloader = Downloader::new();

  loop {
    let query = match Text::new("Search: ").prompt() {
      Ok(q) => q,
      Err(error) => {
        debug!("{}", error);
        continue;
      }
    };

    let (anime_list, search_url) = client.search(&query).await?;
    info!("{:?}", anime_list);

    let anime = match Select::new("Animes", anime_list).prompt() {
      Ok(a) => a,
      Err(error) => {
        debug!("{}", error);
        continue;
      }
    };

    let (episode_list, episode_url) = client.get_episodes(&anime.url, &search_url).await?;
    let episode = match Select::new("Episodes", episode_list).prompt() {
      Ok(e) => e,
      Err(error) => {
        debug!("{}", error);
        continue;
      }
    };

    let hls_url = client.get_episode_hls(&episode.url, &episode_url).await?;

    let filename = format!("{}-{}-{}", anime.title, episode.num, episode.title);
    downloader.start(&hls_url, &filename).await?;
  }

  Ok(())
}
