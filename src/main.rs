use anilife_rs::api::LifeClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let client = LifeClient::new();

  let (anime_list, search_url) = client.search(&String::from("무직전생")).await?;
  println!("{:?}", anime_list);

  let anime = &anime_list[0];
  let (episode_list, episode_url) = client.get_episodes(&anime.url, &search_url).await?;
  println!("{:?}", episode_list);

  let hls_url = client
    .get_episode_hls(&episode_list[0].url, &episode_url)
    .await?;
  println!("{}", hls_url);
  Ok(())
}
