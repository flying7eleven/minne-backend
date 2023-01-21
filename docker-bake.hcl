target "docker-metadata-action" {}
target "build" {
  inherits   = ["docker-metadata-action"]
  context    = "./"
  dockerfile = "Dockerfile"
  platforms  = [
    "linux/amd64", # x86_64 processors
    "linux/arm/v8", # ARMv8, also called AArch64 (Raspberry Pi 4+)
    "linux/arm64", # Apple M1/M2/etc. processors
  ]
}