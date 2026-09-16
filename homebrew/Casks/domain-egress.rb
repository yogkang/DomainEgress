cask "domain-egress" do
  arch arm: "aarch64", intel: "x64"

  version "0.1.2"

  sha256 arm:   "REPLACE_WITH_ARM64_SHA256",
         intel: "REPLACE_WITH_X86_64_SHA256"

  url "https://github.com/yogkang/DomainEgress/releases/download/v#{version}/DomainEgress_#{version}_#{arch}.dmg"

  name "DomainEgress"
  desc "macOS local HTTP/HTTPS and SOCKS5 proxy client"
  homepage "https://github.com/yogkang/DomainEgress"

  depends_on macos: ">= :monterey"

  app "DomainEgress.app"
end
