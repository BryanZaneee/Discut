cask "serein" do
  version "1.0.0-nightly.20261001.52"
  sha256 "9b4329c127fe971b180dc29fc06041cdd73de3c21e43c258cadc8d7b4bc8f0d5"

  url "https://github.com/ViceVerse-cz/Serein/releases/download/v#{version}/serein-v#{version}-macOS-ARM64.zip"
  name "Serein"
  desc "Experimental native Discord client"
  homepage "https://github.com/ViceVerse-cz/Serein"

  depends_on arch: :arm64
  depends_on macos: :sonoma

  app "Serein.app"
end
