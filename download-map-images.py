import os
import re
import subprocess
import urllib
import urllib.parse
from os.path import exists

import requests
from bs4 import BeautifulSoup

BASE_URL = "https://wiki.teamfortress.com"
MAP_LIST_URL = BASE_URL + "/wiki/List_of_maps"
OUTPUT_DIR = "tf2_maps"

# Change this to "convert" if your ImageMagick binary is named that
IMAGEMAGICK_BIN = "magick"

os.makedirs(OUTPUT_DIR, exist_ok=True)


def get_soup(url):
    r = requests.get(url)
    r.raise_for_status()
    return BeautifulSoup(r.text, "html.parser")


def scrape_map_list():
    soup = get_soup(MAP_LIST_URL)

    table = soup.find_all("table", {"class": "wikitable"})[1]
    rows = table.find_all("tr")[1:]  # skip header

    maps = []
    for row in rows:
        cols = row.find_all("td")
        if len(cols) < 2:
            continue

        link_tag = cols[0].find("a")
        map_filename = cols[3].get_text(strip=True)

        if not link_tag or not map_filename:
            continue

        map_page = BASE_URL + link_tag["href"]
        maps.append((map_filename, map_page))

    return maps


def find_infobox_image_url(map_page_url):
    """
    Returns a full URL to the full-size image (not the thumbnail) used in the infobox,
    or None if not found.
    """
    soup = get_soup(map_page_url)

    infobox = soup.find("table", class_="infobox")
    if not infobox:
        print("No infobox found:", map_page_url)
        return None

    # Prefer the <a class="image"> parent if present (gives us the <img> inside)
    image_link = infobox.find("a", class_="image")
    img_tag = None
    if image_link and image_link.find("img"):
        img_tag = image_link.find("img")
    else:
        img_tag = infobox.find("img")

    if not img_tag:
        print("No image tag found in infobox:", map_page_url)
        return None

    src = img_tag.get("src")
    if not src:
        print("Image tag has no src:", map_page_url)
        return None

    # Make src an absolute URL
    img_url = urllib.parse.urljoin(BASE_URL, src)

    # If it's a MediaWiki thumbnail, convert to the original image URL:
    # thumbnail:  https://.../w/images/thumb/a/a1/Filename.png/300px-Filename.png
    # original:   https://.../w/images/a/a1/Filename.png
    parsed = urllib.parse.urlparse(img_url)
    path = parsed.path  # e.g. /w/images/thumb/a/a1/Filename.png/300px-Filename.png

    if "/thumb/" in path:
        # Remove '/thumb/' then drop the last segment (the size-prefixed filename)
        before_thumb, after_thumb = path.split(
            "/thumb/", 1
        )  # keep prefix like /w/images
        # after_thumb is like 'a/a1/Filename.png/300px-Filename.png'
        original_path = before_thumb + "/" + after_thumb.rsplit("/", 1)[0]
        # rebuild full URL (preserve scheme/host from BASE_URL)
        parsed_base = urllib.parse.urlparse(BASE_URL)
        original_url = urllib.parse.urlunparse(
            (
                parsed_base.scheme or "https",
                parsed_base.netloc,
                original_path,
                "",
                "",
                "",
            )
        )
        return original_url
    else:
        # already a non-thumb URL (maybe the page already served original)
        return img_url


def download_image(url, dest_path):
    """
    Downloads `url` (absolute or relative) to dest_path. Raises on HTTP errors.
    """
    full_url = urllib.parse.urljoin(BASE_URL, url)
    print(f"    Downloading: {full_url}")
    r = requests.get(full_url, stream=True)
    r.raise_for_status()
    with open(dest_path, "wb") as f:
        for chunk in r.iter_content(8192):
            if chunk:
                f.write(chunk)


def convert_to_webp(src_path, dest_path):
    # 16:9 crop + resize + options
    cmd = [
        IMAGEMAGICK_BIN,
        "convert",
        src_path,
        "-gravity",
        "center",
        "-crop",
        "16:9",  # auto crops shortest dimension
        "+repage",
        "-resize",
        "640x360",
        "-quality",
        "80",
        "-strip",
        "-define",
        "webp:lossless=false",
        "-define",
        "webp:method=6",
        dest_path,
    ]
    subprocess.run(cmd, check=True)


def main():
    maps = scrape_map_list()
    print(f"Found {len(maps)} maps.")

    for map_filename, map_page in maps:
        if os.path.exists(os.path.join(OUTPUT_DIR, f"{map_filename}.webp")):
            print(f"  ✅ Skipping {map_filename}, already exists.")
            continue
        img_url = find_infobox_image_url(map_page)

        if not img_url:
            print(f"  ❌ No image found for {map_filename}")
            continue

        ext = img_url.split(".")[-1].split("?")[0]
        raw_img_path = os.path.join(OUTPUT_DIR, f"{map_filename}.{ext}")
        final_img_path = os.path.join(OUTPUT_DIR, f"{map_filename}.webp")

        print(f"  Downloading: {img_url}")
        download_image(img_url, raw_img_path)

        print(f"  Converting → {final_img_path}")
        convert_to_webp(raw_img_path, final_img_path)

        os.remove(raw_img_path)

    print("Done!")


if __name__ == "__main__":
    main()
