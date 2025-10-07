package main

import (
	"encoding/base64"
	"encoding/json"
	"fmt"
	"io"
	"log/slog"
	"net/http"
	neturl "net/url"
	"regexp"
	"strings"

	"github.com/PuerkitoBio/goquery"
)

const ANILIFE_URL string = "https://anilife.live"
const USER_AGENT string = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/112.0.0.0 Safari/537.36"
const PLAYER_REGEX string = `"(https:\/\/anilife\.live\/h\/live\?.+)"`
const ALDATA_REGEX string = `var _aldata = '(.+?)'`

type AnilifeTransport struct {
	transport http.RoundTripper
}

func (t *AnilifeTransport) RoundTrip(req *http.Request) (*http.Response, error) {
	req.Header.Add("User-Agent", USER_AGENT)
	if t.transport == nil {
		return http.DefaultTransport.RoundTrip(req)
	}
	return t.transport.RoundTrip(req)
}

type AnilifeClient struct {
	client *http.Client
}

func NewAnilifeClient() *AnilifeClient {
	httpClient := &http.Client{
		Transport: &AnilifeTransport{},
	}
	return &AnilifeClient{
		client: httpClient,
	}
}

type Anime struct {
	Id    string
	Title string
	Url   string
}

func (a Anime) String() string {
	ret := ""
	ret += fmt.Sprintf("[%-4s] %s", a.Id, a.Title)
	return ret
}

type Episode struct {
	Title string
	Url   string
	Num   string
}

func (e Episode) String() string {
	return fmt.Sprintf("%-2s | %s", e.Num, e.Title)
}

func (a *AnilifeClient) Search(query string) ([]*Anime, error) {
	query = neturl.QueryEscape(query)
	url := fmt.Sprintf("%s/search?keyword=%s", ANILIFE_URL, query)
	res, err := a.client.Get(url)
	if err != nil {
		slog.Error("Search", "error", err)
		return nil, err
	}
	doc, err := goquery.NewDocumentFromReader(res.Body)
	if err != nil {
		slog.Error("Search", "error", err)
		return nil, err
	}

	animes := []*Anime{}
	doc.Find(".bsx").Each(func(i int, s *goquery.Selection) {
		url := s.Find("a").AttrOr("href", "")
		title := s.Find("h2[itemprop]").Text()
		urlParts := strings.Split(url, "/")
		id := urlParts[len(urlParts)-1]
		animes = append(animes, &Anime{id, title, url})
	})

	return animes, nil
}

func (a *AnilifeClient) GetAnime(id string) (*Anime, []*Episode, error) {
	url := fmt.Sprintf("%s/detail/id/%s", ANILIFE_URL, id)
	res, err := a.client.Get(url)
	if err != nil {
		slog.Error("GetAnime", "error", err)
		return nil, nil, err
	}
	doc, err := goquery.NewDocumentFromReader(res.Body)
	if err != nil {
		slog.Error("GetAnime", "error", err)
		return nil, nil, err
	}

	title := doc.Find("h1.entry-title").Text()
	anime := &Anime{id, title, res.Request.URL.String()}
	episodes := []*Episode{}
	doc.Find("div.eplister li").Each(func(i int, s *goquery.Selection) {
		title := s.Find(".epl-title").Text()
		num := s.Find(".epl-num").Text()
		path := s.Find("a").AttrOr("href", "")
		url := ANILIFE_URL + path
		episodes = append(episodes, &Episode{title, url, num})
	})

	return anime, episodes, nil
}

type AlData struct {
	VidUrl1080 string `json:"vid_url_1080"`
	VidUrl720  string `json:"vid_url_720"`
}

type VideoData struct {
	Url string `json:"url"`
}

func (a *AnilifeClient) GetEpisodeHLS(episode *Episode, anime *Anime) (string, error) {
	playerUrls, err := a.getPlayerUrls(episode, anime)
	if err != nil {
		slog.Error("GetEpisodeHLS", "error", err)
		return "", err
	}
	if len(playerUrls) == 0 {
		return "", fmt.Errorf("no player url found")
	}

	playerUrl := playerUrls[0]
	aldata, err := a.getAlData(playerUrls[0])
	if err != nil {
		slog.Error("GetEpisodeHLS", "error", err)
		return "", err
	}

	videoUrl := ""
	if aldata.VidUrl1080 != "" {
		videoUrl = "https://" + aldata.VidUrl1080
	} else if aldata.VidUrl720 != "" {
		videoUrl = "https://" + aldata.VidUrl720
	} else {
		return "", fmt.Errorf("hls url not found in aldata")
	}

	req, err := http.NewRequest("GET", videoUrl, nil)
	if err != nil {
		slog.Error("GetEpisodeHLS", "error", err)
		return "", err
	}
	req.Header.Set("Referer", playerUrl)
	res, err := a.client.Do(req)
	if err != nil {
		slog.Error("GetEpisodeHLS", "error", err)
		return "", err
	}
	body, err := io.ReadAll(res.Body)
	if err != nil {
		slog.Error("GetEpisodeHLS", "error", err)
		return "", err
	}
	videoData := []VideoData{}
	err = json.Unmarshal(body, &videoData)
	if err != nil {
		slog.Error("GetEpisodeHLS", "error", err)
		return "", err
	}
	return videoData[0].Url, nil
}

func (a *AnilifeClient) getPlayerUrls(episode *Episode, anime *Anime) ([]string, error) {
	req, err := http.NewRequest("GET", episode.Url, nil)
	if err != nil {
		slog.Error("getPlayerUrls", "error", err)
		return nil, err
	}
	req.Header.Set("Referer", anime.Url)
	res, err := a.client.Do(req)
	if err != nil {
		slog.Error("getPlayerUrls", "error", err)
		return nil, err
	}
	body, err := io.ReadAll(res.Body)
	if err != nil {
		slog.Error("getPlayerUrls", "error", err)
		return nil, err
	}
	html := string(body[:])

	// get player urls from html
	playerRe := regexp.MustCompile(PLAYER_REGEX)
	playerMatches := playerRe.FindAllStringSubmatch(html, -1)
	playerUrls := []string{}
	for _, m := range playerMatches {
		playerUrls = append(playerUrls, m[1])
	}
	return playerUrls, nil
}

func (a *AnilifeClient) getAlData(playerUrl string) (*AlData, error) {
	res, err := a.client.Get(playerUrl)
	if err != nil {
		slog.Error("getAlData", "error", err)
		return nil, err
	}
	body, err := io.ReadAll(res.Body)
	if err != nil {
		slog.Error("getAlData", "error", err)
		return nil, err
	}
	html := string(body[:])

	aldataRe := regexp.MustCompile(ALDATA_REGEX)
	aldataMatches := aldataRe.FindStringSubmatch(html)
	if aldataMatches == nil {
		return nil, fmt.Errorf("no aldata url found")
	}
	aldataString := aldataMatches[1]

	decodedData, err := base64.StdEncoding.DecodeString(aldataString)
	if err != nil {
		slog.Error("getAlData", "error", err)
		return nil, err
	}

	aldata := &AlData{}
	err = json.Unmarshal(decodedData, aldata)
	if err != nil {
		slog.Error("getAlData", "error", err)
		return nil, err
	}
	return aldata, nil
}

