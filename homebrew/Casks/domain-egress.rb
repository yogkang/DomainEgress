cask "domain-egress" do
  arch arm: "aarch64", intel: "x64"

  version "0.2.0"

  sha256 arm:   "71ecd166e4567ec4e327ba600db73f1185532b2cf0827da6f1c4b3f0ebe81a9b",
         intel: "fa26ccb999a1105aa44b49def7b42fd011b62a172125d4404c1f1fdd188e874d"

  url "https://github.com/yogkang/DomainEgress/releases/download/v#{version}/DomainEgress_#{version}_#{arch}.dmg"

  name "DomainEgress"
  desc "macOS local HTTP/HTTPS and SOCKS5 proxy client"
  homepage "https://github.com/yogkang/DomainEgress"

  depends_on macos: :monterey

  app "DomainEgress.app"
end
