package main

import (
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"regexp"
	"sort"
	"strings"
	"sync"

	"github.com/PuerkitoBio/goquery"
)

const anilifeUrl = "https://anilife.live"
const userAgent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36"
const HLS_SEG_TAG = "#EXTINF"

type LifeAnimeInfo struct {
	Id    string
	Title string
	Url   string
}

type LifeAnime struct {
	Info     LifeAnimeInfo
	Episodes []LifeEpisodeInfo
}

type LifeEpisodeInfo struct {
	Title string
	Url   string
	Num   string
}

func GetIdFromUrl(url string) string {
	parts := strings.Split(url, "/")
	return parts[len(parts)-1]
}

func (c *LifeClient) Search(query string) ([]LifeAnimeInfo, error) {
	searchUrl := fmt.Sprintf("%s/search?keyword=%s", anilifeUrl, query)
	res, err := c.Get(searchUrl)
	if err != nil {
		return nil, err
	}
	defer res.Body.Close()

	doc, err := goquery.NewDocumentFromReader(res.Body)
	if err != nil {
		return nil, err
	}

	infos := []LifeAnimeInfo{}
	doc.Find(".bsx").Each(func(i int, s *goquery.Selection) {
		relativeUrl, exists := s.Find("a").Attr("href")
		if !exists {
			return
		}
		url := anilifeUrl + relativeUrl
		title := s.Find("h2").Text()
		id := GetIdFromUrl(url)

		infos = append(infos, LifeAnimeInfo{
			Url:   url,
			Title: title,
			Id:    id,
		})
	})

	return infos, nil
}

func (c *LifeClient) GetAnime(id string) (*LifeAnime, error) {
	animeUrl := fmt.Sprintf("%s/detail/id/%s", anilifeUrl, id)
	res, err := c.Get(animeUrl)
	if err != nil {
		return nil, err
	}
	defer res.Body.Close()

	doc, err := goquery.NewDocumentFromReader(res.Body)
	if err != nil {
		return nil, err
	}

	title := doc.Find(".entry-title").Text()
	if err != nil {
		return nil, err
	}
	info := LifeAnimeInfo{
		Title: title,
		Id:    id,
		Url:   res.Request.URL.String(),
	}

	episodes := []LifeEpisodeInfo{}
	doc.Find(".eplister li").Each(func(i int, s *goquery.Selection) {
		url := s.Find("a").AttrOr("href", "")
		num := s.Find(".epl-num").Text()
		title := s.Find(".epl-title").Text()
		episodes = append(episodes, LifeEpisodeInfo{
			Url:   url,
			Num:   num,
			Title: title,
		})
	})

	anime := LifeAnime{
		Info:     info,
		Episodes: episodes,
	}
	return &anime, nil
}

type PlayerData struct {
	Token string `json:"vid_token_1080"`
	Url   string `json:"vid_url_1080"`
}

type VideoData struct {
	Resolution string `json:"resolution"`
	Url        string `json:"url"`
}

func (c *LifeClient) GetEpisodeHls(url string, referer string) (string, error) {
	defer (func() { c.Referer = "" })()
	c.Referer = referer
	episodeHtmlBytes, err := c.GetBody(url)
	if err != nil {
		return "", err
	}
	episodeHtml := string(episodeHtmlBytes[:])

	playerUrlR := regexp.MustCompile(`"(?<Path>https:\/\/anilife.live\/h\/live\?p=.+)"`)
	matches := playerUrlR.FindAllStringSubmatch(episodeHtml, 1)
	if len(matches) == 0 {
		return "", errors.New("playerUrl not found in episode")
	}
	playerUrl := matches[0][1]
	playerHtmlBytes, err := c.GetBody(playerUrl)
	if err != nil {
		return "", err
	}
	playerHtml := string(playerHtmlBytes[:])

	aldataR := regexp.MustCompile(`var _aldata = '(.+?)'`)
	match := aldataR.FindStringSubmatch(playerHtml)
	if len(match) != 2 {
		return "", errors.New("aldata not found len: " + string(len(match)))
	}
	encodedPlayerData := match[1]
	playerDataBytes, err := base64.StdEncoding.DecodeString(encodedPlayerData)
	if err != nil {
		return "", err
	}
	playerData := PlayerData{}
	err = json.Unmarshal(playerDataBytes, &playerData)
	if err != nil {
		return "", err
	}

	videoUrl := fmt.Sprintf("https://%s", playerData.Url)
	c.Referer = playerUrl
	videoData := []VideoData{}
	err = c.GetJson(videoUrl, &videoData)
	if len(videoData) == 0 {
		return "", errors.New("video data not found")
	}

	return videoData[0].Url, nil
}

func (c *LifeClient) DownloadEpisode(url string, filename string) error {
	err := os.MkdirAll("./segments", 0755)
	if err != nil {
		return err
	}

	var wg sync.WaitGroup
	defer (func() { c.Referer = "" })()

	c.Referer = anilifeUrl
	hlsBytes, err := c.GetBody(url)
	if err != nil {
		return err
	}

	segmentUrls := ParseHls(string(hlsBytes[:]))
	segments := []*Segment{}
	count := 0
	for idx, segmentUrl := range segmentUrls {
		wg.Add(1)
		go func(idx int, url string) {
			// TODO: handle error
			segment, _ := c.DownloadSegment(idx, url)
			count++
			fmt.Printf("%d / %d\n", count, len(segmentUrls))
			segments = append(segments, segment)
			wg.Done()
		}(idx, segmentUrl)
	}
	wg.Wait()

	sort.Slice(segments, func(i, j int) bool {
		return segments[i].Index < segments[j].Index
	})

	os.Remove("./segments/all.ts")

	allTs, err := os.OpenFile("./segments/all.ts", os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0755)
	if err != nil {
		return err
	}
	defer allTs.Close()

	for _, segment := range segments {
		bytes, err := os.ReadFile(segment.FileName)
		if err != nil {
			return err
		}

		_, err = allTs.Write(bytes)
		if err != nil {
			return err
		}
	}

	err = os.Rename("./segments/all.ts", filename)
	if err != nil {
		return err
	}

	os.RemoveAll("./segments")

	return nil
}

type Segment struct {
	Index    int
	FileName string
}

func (c *LifeClient) DownloadSegment(idx int, url string) (*Segment, error) {
	c.Referer = anilifeUrl
	bytes, err := c.GetBody(url)
	if err != nil {
		return nil, err
	}
	filename := fmt.Sprintf("./segments/seg%04d.ts", idx)
	file, err := os.OpenFile(filename, os.O_CREATE|os.O_WRONLY, 0644)
	if err != nil {
		return nil, err
	}
	defer file.Close()

	_, err = file.Write(bytes)
	if err != nil {
		return nil, err
	}
	return &Segment{Index: idx, FileName: filename}, nil
}

func ParseHls(content string) []string {
	lines := strings.Split(content, "\r\n")
	n := len(lines)
	index := 0

	segmentUrls := []string{}
	for index < n {
		line := lines[index]
		if strings.HasPrefix(line, HLS_SEG_TAG) {
			index += 1
			segmentUrl := lines[index]
			segmentUrls = append(segmentUrls, segmentUrl)
		}
		index += 1
	}

	return segmentUrls
}
