package main

import (
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promhttp"
)

func applyMetrics(state *State, versionGauge *prometheus.GaugeVec, outdatedGauge prometheus.Gauge, eolGauge prometheus.Gauge) {
	versionGauge.Reset()
	versionGauge.WithLabelValues(
		state.ServerVersion.String(),
		state.LatestEKSVersion.String(),
		state.LatestK8sVersion.String(),
		state.EOLK8sVersion.String(),
		state.CurrentTime,
		state.CurrentTimeText,
	).Set(0)

	outdatedGauge.Set(state.IsOutdated)
	eolGauge.Set(state.IsPastEOL)
}

func main() {
	log.SetFlags(log.LstdFlags)

	state, err := NewState()
	if err != nil {
		log.Fatalf("failed to initialize state: %v", err)
	}

	log.Printf("server: %s", state.ServerVersion.String())
	log.Printf("latest EKS: %s", state.LatestEKSVersion.String())
	log.Printf("latest k8s release: %s", state.LatestK8sVersion.String())
	log.Printf("oldest k8s release supported: %s", state.EOLK8sVersion.String())

	if state.LatestEKSVersion.GreaterThan(state.ServerVersion) {
		log.Printf("Server version %s is outdated", state.ServerVersion.String())
	}

	versionGauge := prometheus.NewGaugeVec(
		prometheus.GaugeOpts{
			Name: "eks_version_exporter",
			Help: "Bunch of values",
		},
		[]string{
			"server_current_version",
			"eks_latest_available_version",
			"k8s_latest_available_version",
			"eol_latest_available_version",
			"last_updated",
			"last_updated_text",
		},
	)

	outdatedGauge := prometheus.NewGauge(
		prometheus.GaugeOpts{
			Name: "eks_version_exporter_is_outdated",
			Help: "If value is 1 then cluster version is outdated",
		},
	)

	eolGauge := prometheus.NewGauge(
		prometheus.GaugeOpts{
			Name: "eks_version_exporter_is_past_eol",
			Help: "If value is 1 then cluster version is older than EOL",
		},
	)

	registry := prometheus.NewRegistry()
	registry.MustRegister(versionGauge, outdatedGauge, eolGauge)

	applyMetrics(state, versionGauge, outdatedGauge, eolGauge)

	mux := http.NewServeMux()
	mux.Handle("/metrics", promhttp.HandlerFor(registry, promhttp.HandlerOpts{}))

	server := &http.Server{
		Addr:              "0.0.0.0:8080",
		Handler:           mux,
		ReadHeaderTimeout: 5 * time.Second,
	}

	go func() {
		log.Printf("Prometheus exporter listening on 0.0.0.0:8080")
		if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			log.Fatalf("http server failed: %v", err)
		}
	}()

	ticker := time.NewTicker(30 * time.Minute)
	defer ticker.Stop()

	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)

	for {
		select {
		case <-ticker.C:
			log.Printf("%s: Updating metrics", currentTimeDateString())
			if err := state.Refresh(); err != nil {
				log.Printf("refresh failed: %v", err)
				continue
			}

			log.Printf("server: %s", state.ServerVersion.String())
			log.Printf("latest EKS: %s", state.LatestEKSVersion.String())
			log.Printf("latest k8s release: %s", state.LatestK8sVersion.String())
			log.Printf("oldest k8s release supported: %s", state.EOLK8sVersion.String())

			applyMetrics(state, versionGauge, outdatedGauge, eolGauge)
		case <-quit:
			log.Printf("Shutting down")
			_ = server.Close()
			return
		}
	}
}
