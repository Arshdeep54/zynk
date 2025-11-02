TAG ?= v$$(date +%Y-%m-%d-%H%M)
IMAGE := zynk-kv:$(TAG)
KUBECTL ?= minikube kubectl --


.PHONY: build load set-image deploy all

build:
	docker build -t $(IMAGE) .

load:
	minikube image load $(IMAGE)

set-image:
	$(KUBECTL) set image deploy/zynk-lb zynk-lb=$(IMAGE)
	$(KUBECTL) set image statefulset/zynkd zynkd=$(IMAGE)

deploy:
	$(KUBECTL) apply -k deploy/k8s

all: build load set-image