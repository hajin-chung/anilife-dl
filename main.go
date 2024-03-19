package main

import (
	"flag"
	"fmt"
	"log"
	"os"
	"strconv"
	"strings"
)

type CommandType string

var client *LifeClient

func main() {
	client = NewLifeClient()

	searchCmd := flag.NewFlagSet("search", flag.ExitOnError)
	downloadCmd := flag.NewFlagSet("download", flag.ExitOnError)
	infoCmd := flag.NewFlagSet("info", flag.ExitOnError)

	downloadEpisodes := downloadCmd.String("episodes", "", "")

	if len(os.Args) < 2 {
		printHelp()
		return
	}

	switch os.Args[1] {
	case "search":
		handleSearch(searchCmd)
	case "info":
		handleInfo(infoCmd)
	case "download":
		handleDownload(downloadCmd, downloadEpisodes)
	default:
		printHelp()
	}
}

func printHelp() {
	fmt.Println("anilfie-dl")
	fmt.Println("Usage:")
	fmt.Println("  anilife-dl search <query>")
	fmt.Println("  anilife-dl info <anime id>")
	fmt.Println("  anilife-dl download <anime id>")
	fmt.Println("  anilife-dl download --episodes <episode number>,... <anime id>")
}

func handleSearch(searchCmd *flag.FlagSet) {
	if err := searchCmd.Parse(os.Args[2:]); err != nil {
		fmt.Println("Failed to parse search command")
		os.Exit(1)
	}
	if searchCmd.NArg() != 1 {
		fmt.Println("You must provide a search query")
		os.Exit(1)
	}
	query := searchCmd.Arg(0)
	fmt.Printf("Searching for: %s\n", query)
	infos, err := client.Search(query)
	for _, info := range infos {
		fmt.Printf("%4s | %s\n", info.Id, info.Title)
	}
	if err != nil {
		log.Fatal(err)
	}
}

func handleInfo(infoCmd *flag.FlagSet) {
	if err := infoCmd.Parse(os.Args[2:]); err != nil {
		fmt.Println("Failed to parse info command")
		os.Exit(1)
	}
	if infoCmd.NArg() != 1 {
		fmt.Println("You must provide a anime id")
		os.Exit(1)
	}

	animeId := infoCmd.Arg(0)

	anime, err := client.GetAnime(animeId)
	if err != nil {
		fmt.Printf("error on GetAnime: %s", err)
		os.Exit(1)
	}

	fmt.Printf("# %s\n", anime.Info.Title)
	for _, episode := range anime.Episodes {
		fmt.Printf("%3s | %s\n", episode.Num, episode.Title)
	}
	if err != nil {
		log.Fatal(err)
	}
}

func handleDownload(downloadCmd *flag.FlagSet, targetEpisodes *string) {
	if err := downloadCmd.Parse(os.Args[2:]); err != nil {
		fmt.Println("Failed to parse download command")
		os.Exit(1)
	}
	if downloadCmd.NArg() != 1 {
		fmt.Println("You must provide an anime ID")
		os.Exit(1)
	}

	animeId := downloadCmd.Arg(0)

	fmt.Printf("Info on anime %s\n", animeId)
	anime, err := client.GetAnime(animeId)
	if err != nil {
		fmt.Printf("error on GetAnime: %s\n", err)
		os.Exit(1)
	}

	os.MkdirAll(fmt.Sprintf("./%s", anime.Info.Title), 0755)
	fmt.Printf("Downloading %s\n", anime.Info.Title)
	if *targetEpisodes == "" {
		for _, episode := range anime.Episodes {
			fmt.Printf("Downloading %s %s\n", episode.Num, episode.Title)
			hlsUrl, err := client.GetEpisodeHls(episode.Url, anime.Info.Url)
			if err != nil {
				fmt.Printf("error on GetEpisodeHls: %s\n", err)
				os.Exit(1)
			}

			episodeFileName := fmt.Sprintf("./%s/%02s-%s.ts", anime.Info.Title, episode.Num, episode.Title)
			err = client.DownloadEpisode(hlsUrl, episodeFileName)
			if err != nil {
				fmt.Printf("error on DownloadEpisode: %s\n", err)
				os.Exit(1)
			}
		}
	} else {
		fmt.Printf("Episodes: %s\n", *targetEpisodes)
		eps := strings.Split(*targetEpisodes, ",")
		fmt.Printf("Downloading episodes: %v\n", eps)

		for _, epi := range eps {
			episodeIdx, err := strconv.Atoi(epi)
			if err != nil {
				fmt.Printf("error on strconv epi: %s\n", err)
				os.Exit(1)
			}

			if episodeIdx < 0 || len(anime.Episodes) <= episodeIdx {
				fmt.Println("episode index is out of bounds")
				continue
			}

			episode := anime.Episodes[episodeIdx]
			fmt.Printf("Downloading %s %s\n", episode.Num, episode.Title)
			hlsUrl, err := client.GetEpisodeHls(episode.Url, anime.Info.Url)
			if err != nil {
				fmt.Printf("error on GetEpisodeHls: %s\n", err)
				os.Exit(1)
			}

			episodeFileName := fmt.Sprintf("./%s/%02s-%s", episode.Num, episode.Title)
			err = client.DownloadEpisode(hlsUrl, episodeFileName)
			if err != nil {
				fmt.Printf("error on DownloadEpisode: %s\n", err)
				os.Exit(1)
			}

		}
	}
}
