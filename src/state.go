package main

import (
	"encoding/json"
	"encoding/xml"
	"fmt"
	"io"
	"net/http"
	"os/exec"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"time"
)

const (
	awsEKSDocHistoryURL = "https://docs.aws.amazon.com/eks/latest/userguide/doc-history.rss"
	k8sReleasesAtomURL  = "https://github.com/kubernetes/kubernetes/releases.atom"
)

var eksVersionRegex = regexp.MustCompile(`Kubernetes version (\d+\.\d+)`)

type Version struct {
	Major int
	Minor int
	Patch int
}

func (v Version) String() string {
	return fmt.Sprintf("%d.%d.%d", v.Major, v.Minor, v.Patch)
}

func (v Version) GreaterThan(other Version) bool {
	if v.Major != other.Major {
		return v.Major > other.Major
	}
	if v.Minor != other.Minor {
		return v.Minor > other.Minor
	}
	return v.Patch > other.Patch
}

func parseVersion(raw string) (Version, bool, error) {
	cleaned := strings.TrimSpace(raw)
	if cleaned == "" {
		return Version{}, false, fmt.Errorf("empty version")
	}

	if strings.HasPrefix(cleaned, "v") {
		cleaned = cleaned[1:]
	}

	hasPrerelease := strings.Contains(cleaned, "-")

	core := cleaned
	if idx := strings.IndexAny(cleaned, "-+"); idx >= 0 {
		core = cleaned[:idx]
	}

	parts := strings.Split(core, ".")
	if len(parts) < 2 {
		return Version{}, hasPrerelease, fmt.Errorf("invalid version: %q", raw)
	}

	major, err := strconv.Atoi(parts[0])
	if err != nil {
		return Version{}, hasPrerelease, fmt.Errorf("invalid major in %q: %w", raw, err)
	}

	minor, err := strconv.Atoi(parts[1])
	if err != nil {
		return Version{}, hasPrerelease, fmt.Errorf("invalid minor in %q: %w", raw, err)
	}

	patch := 0
	if len(parts) >= 3 {
		patch, err = strconv.Atoi(parts[2])
		if err != nil {
			return Version{}, hasPrerelease, fmt.Errorf("invalid patch in %q: %w", raw, err)
		}
	}

	return Version{Major: major, Minor: minor, Patch: patch}, hasPrerelease, nil
}

type kubectlVersionResponse struct {
	ServerVersion struct {
		GitVersion string `json:"gitVersion"`
	} `json:"serverVersion"`
}

type eksRSS struct {
	Channel struct {
		Items []struct {
			Title string `xml:"title"`
		} `xml:"item"`
	} `xml:"channel"`
}

type atomFeed struct {
	Entries []struct {
		ID string `xml:"id"`
	} `xml:"entry"`
}

type State struct {
	ServerVersion    Version
	LatestEKSVersion Version
	LatestK8sVersion Version
	EOLK8sVersion    Version
	CurrentTime      string
	CurrentTimeText  string
	IsOutdated       float64
	IsPastEOL        float64
}

func NewState() (*State, error) {
	s := &State{}
	if err := s.Refresh(); err != nil {
		return nil, err
	}
	return s, nil
}

func (s *State) Refresh() error {
	serverVersion, err := getServerK8sVersion()
	if err != nil {
		return err
	}

	latestEKSVersion, err := getLatestEKSK8sVersion()
	if err != nil {
		return err
	}

	latestK8sVersion, err := getLatestK8sVersion()
	if err != nil {
		return err
	}

	eolMinor := latestK8sVersion.Minor - 2
	if eolMinor < 0 {
		eolMinor = 0
	}

	s.ServerVersion = serverVersion
	s.LatestEKSVersion = latestEKSVersion
	s.LatestK8sVersion = latestK8sVersion
	s.EOLK8sVersion = Version{Major: latestK8sVersion.Major, Minor: eolMinor, Patch: 0}
	s.CurrentTime = strconv.FormatInt(currentTimeEpochMS(), 10)
	s.CurrentTimeText = currentTimeDateString()

	if s.LatestEKSVersion.GreaterThan(s.ServerVersion) {
		s.IsOutdated = 1
	} else {
		s.IsOutdated = 0
	}

	if s.EOLK8sVersion.GreaterThan(s.ServerVersion) {
		s.IsPastEOL = 1
	} else {
		s.IsPastEOL = 0
	}

	return nil
}

func currentTimeEpochMS() int64 {
	return time.Now().UnixMilli()
}

func currentTimeDateString() string {
	return time.Now().UTC().Format("2006-01-02 15:04:05")
}

func getServerK8sVersion() (Version, error) {
	output, err := exec.Command("kubectl", "version", "-ojson").Output()
	if err != nil {
		return Version{}, fmt.Errorf("kubectl version failed: %w", err)
	}

	var parsed kubectlVersionResponse
	if err := json.Unmarshal(output, &parsed); err != nil {
		return Version{}, fmt.Errorf("unable to parse kubectl json: %w", err)
	}

	if parsed.ServerVersion.GitVersion == "" {
		return Version{}, fmt.Errorf("no serverVersion.gitVersion in kubectl output")
	}

	v, _, err := parseVersion(parsed.ServerVersion.GitVersion)
	if err != nil {
		return Version{}, fmt.Errorf("unable to parse server version: %w", err)
	}

	return Version{Major: v.Major, Minor: v.Minor, Patch: 0}, nil
}

func getLatestEKSK8sVersion() (Version, error) {
	body, err := fetchURL(awsEKSDocHistoryURL)
	if err != nil {
		return Version{}, err
	}

	var rss eksRSS
	if err := xml.Unmarshal(body, &rss); err != nil {
		return Version{}, fmt.Errorf("unable to parse AWS RSS: %w", err)
	}

	versions := make([]Version, 0, len(rss.Channel.Items))
	for _, item := range rss.Channel.Items {
		match := eksVersionRegex.FindStringSubmatch(item.Title)
		if len(match) < 2 {
			continue
		}

		v, _, err := parseVersion(match[1])
		if err != nil {
			continue
		}

		versions = append(versions, Version{Major: v.Major, Minor: v.Minor, Patch: 0})
	}

	if len(versions) == 0 {
		return Version{}, fmt.Errorf("unable to parse EKS Kubernetes versions from AWS RSS")
	}

	sort.Slice(versions, func(i, j int) bool {
		return versions[i].GreaterThan(versions[j])
	})

	return versions[0], nil
}

func getLatestK8sVersion() (Version, error) {
	body, err := fetchURL(k8sReleasesAtomURL)
	if err != nil {
		return Version{}, err
	}

	var feed atomFeed
	if err := xml.Unmarshal(body, &feed); err != nil {
		return Version{}, fmt.Errorf("unable to parse k8s atom feed: %w", err)
	}

	releases := make([]Version, 0, len(feed.Entries))
	for _, entry := range feed.Entries {
		parts := strings.Split(entry.ID, "/")
		if len(parts) == 0 {
			continue
		}

		tag := parts[len(parts)-1]
		v, prerelease, err := parseVersion(tag)
		if err != nil || prerelease {
			continue
		}

		releases = append(releases, v)
	}

	if len(releases) == 0 {
		return Version{}, fmt.Errorf("unable to parse stable Kubernetes versions from GitHub feed")
	}

	sort.Slice(releases, func(i, j int) bool {
		return releases[i].GreaterThan(releases[j])
	})

	return releases[0], nil
}

func fetchURL(url string) ([]byte, error) {
	req, err := http.NewRequest(http.MethodGet, url, nil)
	if err != nil {
		return nil, fmt.Errorf("unable to create request for %s: %w", url, err)
	}
	req.Header.Set("User-Agent", "curl/7.77.0")

	client := &http.Client{Timeout: 30 * time.Second}
	resp, err := client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("request failed for %s: %w", url, err)
	}
	defer resp.Body.Close()

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return nil, fmt.Errorf("request failed for %s with status %s", url, resp.Status)
	}

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("unable to read response body for %s: %w", url, err)
	}

	return body, nil
}
