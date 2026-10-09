cask "domain-egress" do
  arch arm: "aarch64", intel: "x64"

  version "0.5.0"

  sha256 arm:   "0c440128e33b5de6647b01ac65b196759f01a268b15bb7636d458293141d8c4b",
         intel: "f8cb1f437948ab09de170d5d53a5859b161d1485a56fce8dcb0e118c21556344"

  url "https://github.com/yogkang/DomainEgress/releases/download/v#{version}/DomainEgress_#{version}_#{arch}.dmg"

  name "DomainEgress"
  desc "macOS local HTTP/HTTPS and SOCKS5 proxy client"
  homepage "https://github.com/yogkang/DomainEgress"

  depends_on macos: :monterey

  app "DomainEgress.app"
end
