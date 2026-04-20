import json
import urllib.parse
import urllib.request
from typing import Any, Dict, Optional


def _get(url: str, headers: Optional[Dict[str, str]] = None, timeout: int = 20) -> Dict[str, Any]:
    req = urllib.request.Request(url, headers=headers or {"User-Agent": "data-api/1.0"})
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        data = resp.read()
        ct = resp.headers.get("Content-Type", "")
        if "application/json" in ct or url.endswith("json") or url.endswith("JSON"):
            return json.loads(data.decode("utf-8"))
        return {"content": data.decode("utf-8")}


def openalex_search(query: str, per_page: int = 5) -> Dict[str, Any]:
    q = urllib.parse.quote(query)
    url = f"https://api.openalex.org/works?search={q}&per_page={per_page}"
    return _get(url)


def crossref_search(query: str, rows: int = 5) -> Dict[str, Any]:
    q = urllib.parse.quote(query)
    url = f"https://api.crossref.org/works?query={q}&rows={rows}"
    return _get(url)


def arxiv_search(query: str, start: int = 0, max_results: int = 5) -> Dict[str, Any]:
    q = urllib.parse.quote(query)
    url = f"http://export.arxiv.org/api/query?search_query=all:{q}&start={start}&max_results={max_results}"
    return _get(url, headers={"Accept": "application/atom+xml"})


def pubmed_esearch(term: str) -> Dict[str, Any]:
    q = urllib.parse.quote(term)
    url = f"https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=pubmed&retmode=json&term={q}"
    return _get(url)


def worldbank_indicator(country_code: str, indicator: str) -> Dict[str, Any]:
    url = f"http://api.worldbank.org/v2/country/{country_code}/indicator/{indicator}?format=json"
    return _get(url)


def fred_observations(series_id: str, api_key: str) -> Dict[str, Any]:
    sid = urllib.parse.quote(series_id)
    key = urllib.parse.quote(api_key)
    url = f"https://api.stlouisfed.org/fred/series/observations?series_id={sid}&api_key={key}&file_type=json"
    return _get(url)


def wikidata_sparql(query: str) -> Dict[str, Any]:
    data = urllib.parse.urlencode({"query": query}).encode("utf-8")
    req = urllib.request.Request(
        "https://query.wikidata.org/sparql",
        data=data,
        headers={"Accept": "application/sparql-results+json", "User-Agent": "data-api/1.0"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=30) as resp:
        return json.loads(resp.read().decode("utf-8"))


def pubchem_cid_by_name(name: str) -> Dict[str, Any]:
    q = urllib.parse.quote(name)
    url = f"https://pubchem.ncbi.nlm.nih.gov/rest/pug/compound/name/{q}/cids/JSON"
    return _get(url)


def uniprot_search(query: str) -> Dict[str, Any]:
    q = urllib.parse.quote(query)
    url = f"https://rest.uniprot.org/uniprotkb/search?query={q}"
    return _get(url)


def opencorporates_search(q: str) -> Dict[str, Any]:
    s = urllib.parse.quote(q)
    url = f"https://api.opencorporates.com/companies/search?q={s}"
    return _get(url)


def overpass_query(query: str) -> Dict[str, Any]:
    data = urllib.parse.urlencode({"data": query}).encode("utf-8")
    req = urllib.request.Request(
        "https://overpass-api.de/api/interpreter",
        data=data,
        headers={"Content-Type": "application/x-www-form-urlencoded", "User-Agent": "data-api/1.0"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=60) as resp:
        ct = resp.headers.get("Content-Type", "")
        body = resp.read().decode("utf-8")
        if "application/json" in ct:
            return json.loads(body)
        return {"content": body}
