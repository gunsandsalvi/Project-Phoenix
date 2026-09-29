"""Where each profile primitive lives, and the one way the derivations write and read them.

A profile is four files by what they describe, and `later/`, the tables of systems a later stage builds, each in a
file of its system's code, which a world takes only with its system. Every derivation writes its primitives by id
through `put`, which replaces an entry of the same id in place or appends it, so no derivation owns a file and a
rerun of one leaves the others' entries as they were. `get` finds a primitive wherever its file is.
"""

import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PROFILES = ROOT / "data" / "profiles"

HEADERS = {
    "economy": ("The {level} group's accounts of 2019 (spec GEN.15): the ways, the price levels, the flows' "
                "primitives, the balance sheet and real assets, and the shapes the opening scales to them. Written "
                "by tools/data's derivations; never edited by hand."),
    "people": ("The {level} group's people (spec GEN.2): demography, households and education, the labour force, "
               "the wealth shape and the households' banking. Written by tools/data's derivations; never edited by "
               "hand."),
    "law": ("The {level} group's law (spec GEN.1, NUM.3): every policy a country's law sets. Written by tools/data's "
            "derivations; never edited by hand."),
    "profile": ("The {level} group's profile (spec GEN.15): the joint draw of the values a country is drawn with. "
                "Written by tools/data/derive.py; never edited by hand."),
}

ECONOMY = {
    "TEC.inputs", "TEC.labour", "TEC.capital", "TEC.land", "GDS.price_level", "GEN.service_inputs",
    "GEN.product_taxes", "GEN.value_added_parts", "GEN.final_weights", "GEN.final_composition", "GEN.balance_sheet",
    "GEN.real_assets", "GEN.productivity_spread", "GEN.occupation_pay", "CAP.stock_per_gdp",
    "FRM.firms_per_employed", "SOC.public_staff_share",
}
LAW = {
    "CB.currency", "GEN.units_per_dollar", "DEM.age_of_majority", "DEM.school_leaving_age",
    "FRM.insolvency_grace_days", "IDX.base", "LAB.full_time_hours", "LAB.notice_days", "LAB.severance_days_a_year",
    "LAB.minimum_wage_share", "SOC.benefit_replacement", "SOC.benefit_months", "SOC.pension_age",
    "SOC.replacement_rate", "SOC.pension_coverage", "SOC.disability_benefit_coverage", "STA.release_day",
    "STA.release_lag", "STA.sample_share", "STA.early_returns", "STA.revision_months", "TAX.income_band_edges",
    "TAX.income_band_rates", "TAX.consumption_rate", "TIME.calendar", "TRS.payment_order",
}
PROFILE = {"GEN.profile"}
LATER = {"HSG", "PEN"}


def file_of(pid: str) -> Path:
    """The file a primitive lives in, relative to its level's directory."""
    system = pid.split(".")[0]
    if system in LATER:
        return Path("later") / f"{system}.toml"
    if pid in ECONOMY:
        return Path("economy.toml")
    if pid in LAW:
        return Path("law.toml")
    if pid in PROFILE:
        return Path("profile.toml")
    return Path("people.toml")


def _blocks(text: str) -> tuple:
    """A file's header and its entries, each entry's text from its `[[primitive]]` on."""
    head, *rest = text.split("\n[[primitive]]\n")
    return head, ["[[primitive]]\n" + r.rstrip("\n") for r in rest]


def _id(block: str) -> str:
    return tomllib.loads(block)["primitive"][0]["id"]


def put(level: str, entries: list) -> None:
    """Each entry, a `[[primitive]]` block's text, written into its file: in place of an entry of its id, or after
    the file's last."""
    by_file = {}
    for e in entries:
        block = e if isinstance(e, str) else "\n".join(e)
        block = block.strip("\n")
        if not block.startswith("[[primitive]]"):
            block = "[[primitive]]\n" + block
        by_file.setdefault(file_of(_id(block)), []).append(block)
    for rel, blocks in by_file.items():
        path = PROFILES / level / rel
        if path.exists():
            head, old = _blocks(path.read_text())
        else:
            name = rel.stem if rel.parent == Path(".") else None
            header = HEADERS[name] if name else (f"The {{level}} group's {rel.stem} tables, for the system a later "
                                                 "stage builds; a world takes them only with it. Written by "
                                                 "tools/data's derivations; never edited by hand.")
            head, old = "# " + header.format(level=level), []
        ids = [_id(b) for b in old]
        for b in blocks:
            pid = _id(b)
            if pid in ids:
                old[ids.index(pid)] = b
            else:
                old.append(b)
                ids.append(pid)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(head.rstrip("\n") + "\n\n" + "\n\n".join(old) + "\n")


def get(level: str, pid: str):
    """A primitive's value, read from its file."""
    path = PROFILES / level / file_of(pid)
    for p in tomllib.loads(path.read_text()).get("primitive", []):
        if p["id"] == pid:
            return p["value"]
    raise KeyError(f"{pid} is not in {path}")


def entry(level: str, pid: str) -> dict:
    """A primitive's whole entry, read from its file."""
    path = PROFILES / level / file_of(pid)
    for p in tomllib.loads(path.read_text()).get("primitive", []):
        if p["id"] == pid:
            return p
    raise KeyError(f"{pid} is not in {path}")


def put_text(level: str, text: str) -> None:
    """Each entry of a file's text, its header aside, written into its file by `put`."""
    _, blocks = _blocks(text)
    put(level, blocks)


def retire(level: str, ids: list) -> None:
    """Each primitive of `ids` taken out of its file, where it is there."""
    for rel in {file_of(pid) for pid in ids}:
        path = PROFILES / level / rel
        if not path.exists():
            continue
        head, blocks = _blocks(path.read_text())
        kept = [b for b in blocks if _id(b) not in ids]
        path.write_text(head.rstrip("\n") + "\n\n" + "\n\n".join(kept) + "\n")
