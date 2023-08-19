use anilife_rs::{api::LifeClient, dl::Downloader};
#[macro_use]
extern crate log;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  env_logger::init();

  let client = LifeClient::new();
  let downloader = Downloader::new();

  let (anime_list, search_url) = client.search(&String::from("무직전생")).await?;
  info!("{:?}", anime_list);

  let anime = &anime_list[0];
  let (episode_list, episode_url) = client.get_episodes(&anime.url, &search_url).await?;
  info!("{:?}", episode_list);

  let hls_url = client
    .get_episode_hls(&episode_list[0].url, &episode_url)
    .await?;
  info!("{}", hls_url);

  downloader.start(&hls_url).await?;

  Ok(())
}
