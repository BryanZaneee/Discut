# Discut screenshots

The Discut images are reviewed native-renderer captures with offline synthetic data.
They do not show a live Discord account or establish working audio/network calls.
Other images in this directory are inherited upstream material.

Reproduce the light-theme Discut set on a graphical desktop:

```sh
scripts/discut-build.sh build --locked --no-default-features --features demo -p serein
scripts/discut-screenshots.sh
```

The script opens and stops only its own fixture processes. It overwrites the three
Discut PNGs; inspect the results before committing them. Keep these reproducible fixtures
separate from any live screenshots. Live images require the account owner to designate
the conversation for public display; review the captured frame before publication and
label its provenance. A display's scale factor affects pixel dimensions. The existing Mac Retina
captures retain native resolution (2240×1520) for readable UI text and are below 1 MiB
each, an intentional exception to the preferred 300 KB repository-image target.
