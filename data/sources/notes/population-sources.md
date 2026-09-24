# S0.25 opening population: published sources, tested 2026-09-24

The groups follow `data/sources/raw/wb/countries.csv`: **dev** = High income (86 economies), **emg** = Upper middle
income (59), **dvp** = Lower middle + Low income (72). "Coverage" is the number of economies in each group with at
least one non-empty observation in 2010–2025. I got every count by downloading the data and matching it to
countries.csv by ISO3 (Eurostat ISO2 codes mapped, with EL→GR, UK→GB and XK→XKX). Samples are in
`scratchpad/sources/`. Files over about 100 MB were counted and then deleted: WID bulk (882 MB), OECD FPS (423 MB),
OECD tenure (178 MB) and several Eurostat files.

Access notes (tested):
- **ILOSTAT**: `rplumber.ilo.org` returns HTTP 200 with an **empty body** for every data and metadata query. The
  ILOSTAT **SDMX API `sdmx.ilo.org` works**. For example:
  `https://sdmx.ilo.org/rest/data/ILO,DF_<INDICATOR>,1.0/all?startPeriod=2010` with header
  `Accept: application/vnd.sdmx.data+csv;version=1.0.0`. It needs as many key positions as the dataflow has
  dimensions, or `all`. The `www.ilo.org/ilostat-files` bulk files return 301 then 404. `ilostat.ilo.org` returns
  403 (Cloudflare).
- **OECD**: `webfs.oecd.org/Els-com/...` serves the Affordable Housing Database xlsx files (200). The AHD is **not**
  an SDMX dataflow: none of the 1,547 dataflows matched. `www.oecd.org` pages return 403 or 404.
- **UN**: `www.un.org` returns 403. The Population Division apps on `population.un.org` serve their xlsx files from
  `/assets/`.
- **Blocked**: `www.ssa.gov` (SSPTW), `www.issa.int`, `www.social-protection.org` (ILO WSPDB) all return 403. The
  `data.un.org` download handler returns the site's HTML home page instead of a zip.
- The WHO GHO API `GHE_YLDRATE` holds only regional and income-group aggregates, with no country rows.

---

## 1. Population by single age and sex; life tables

| Dataset / publisher | URL (tested 200/206) | Format | Measures | Coverage 2010+ (dev/emg/dvp) | Licence |
|---|---|---|---|---|---|
| UN DESA WPP 2024, Population by single age and sex, Medium, 1950–2023 | `https://population.un.org/wpp/assets/Excel%20Files/1_Indicator%20(Standard)/CSV_FILES/WPP2024_PopulationBySingleAgeSex_Medium_1950-2023.csv.gz` (62 MB) | CSV.gz; `ISO3_code, Time, AgeGrp (0…99, 100+), PopMale, PopFemale, PopTotal` (thousands, 1 July) | De facto population by single year of age 0–100+ and sex, per location and year | **85/86, 59/59, 72/72**. The only one missing is CHI (Channel Islands), which has no ISO3 in WPP. | Not stated in the file; the site is a JS app and its terms page could not be fetched |
| same, 2024–2100 projections | `…/CSV_FILES/WPP2024_PopulationBySingleAgeSex_Medium_2024-2100.csv.gz` | same | Medium-variant projection | same set (206 on a range request) | – |
| UN DESA WPP 2024, Complete life table, Medium, both sexes 1950–2023 (plus `_Female_`, `_Male_`) | `…/CSV_FILES/WPP2024_Life_Table_Complete_Medium_Both_1950-2023.csv.gz` (200 MB each) | CSV.gz; `SexID, AgeGrp 0…100+, mx, qx, px, lx, dx, Lx, Sx, Tx, ex, ax` | Single-age period life table. `mx` is the central death rate; `qx` is the probability of dying between age x and x+1. | **85/86, 59/59, 72/72** | – |
| WPP2024 fertility by single age | `…/CSV_FILES/WPP2024_Fertility_by_Age1.csv.gz` | CSV.gz | Age-specific fertility | 206 (not downloaded) | – |

Gap: none. WPP values are the UN's estimates, modelled from censuses, surveys and registration, not raw counts.

## 2. Household composition

| Dataset / publisher | URL | Format | Measures | Coverage 2010+ |
|---|---|---|---|---|
| **UN DESA Database on Household Size and Composition 2026** (UN DESA/POP/2026/DC/NO.15, data processed to 30 Jun 2026; supersedes the 2022 edition) | `https://population.un.org/household/assets/UNDESA_PD_2026_hh-size-composition.xlsx` (399 KB). The www.un.org 2022 URL returns 403. | xlsx, sheet "HH size and composition 2026"; one row per country × source × reference year; ISO3; ".." = missing | Households by size (1, 2–3, 4–5, 6+ members; mean size). **Basic types, as % of households**: one-person; couple only; couple with children (of any age); single parent with children (mother/father); extended family (all related, not nuclear); non-relatives; unknown. **Intergenerational**: nuclear, multi-generation (two or more generations aged 20+), three-generation, skip-generation. Also households with members aged under 15/18/20 and 60+/65+, and female head. Universe: usual residents of private households. Sources: DHS, MICS, IPUMS-I, DYB, EU-LFS. | size / one-person: **64/86, 49/59, 63/72**; couple only and couple with children: **55–56, 48, 63**; extended, non-relatives, multi-generation: **17–18, 48, 63**. In the developed group the detailed types come only from IPUMS/DHS, because DYB tables give no extended or intergenerational types. |
| **UN DESA Database on Households and Living Arrangements of Older Persons 2026** (UN DESA/POP/2026/DC/NO.16) | `https://population.un.org/LivingArrangements/assets/UNDESA_PD_2026_living-arrangements-older-persons.xlsx` (2 MB) | xlsx; rows by country × source × year × age range (60+, 65+, 80+) × sex | % of older persons by household size and type; **co-residing with a spouse; co-residing with children aged 20+ (oldest co-resident child ≥ 20, including children-in-law)**; with children under 20; multi-generation, three-generation and skip-generation | size: **55/86, 49/59, 63/72**; with children 20+: **16/86, 48/59, 63/72** |
| DHS Program API | `https://api.dhsprogram.com/rest/dhs/data?indicatorIds=HC_MEMB_H_MNM&surveyYearStart=2010&breakdown=national&perpage=5000&f=json` (country ISO3 from `/rest/dhs/countries`) | JSON | `HC_MEMB_H_1MM…9MM` (households by number of usual members), `HC_MEMB_H_MNM` (mean size), `HC_OLDR_H_3GN` (3-generation households), `HC_OLDR_H_W65/O65`, `HC_HHHD_H_FEM`. The API has no couple/single-parent typology. | **0/86, 13/59, 48/72** |
| Eurostat `lfst_hhnhtych` "Private households by household composition, number of children and age of youngest child" (LFS) | `https://ec.europa.eu/eurostat/api/dissemination/sdmx/2.1/data/lfst_hhnhtych/?format=SDMX-CSV&startPeriod=2010` | SDMX-CSV; thousands of households; `phhcomp` A1, CPL_CH/NCH, A1_CH, OTH… | Counts of households by type | **29/86, 5/59, 0/72** |
| Eurostat `ilc_lvph02` "Distribution of households by household type" and `ilc_lvph03` "…by household size" (EU-SILC) | same pattern | SDMX-CSV, % | Types: A1 (split <65 and ≥65), A2, A2 with 1/2/3+ children, single with children, A_GE3 (3+ adults, with or without children) | **31/86, 6/59, 0/72** |
| Eurostat `ilc_lvps30` "Distribution of population aged 65 and over by type of household" | same pattern | % | Living alone, couple without others, couple with others, other | **31/86, 6/59, 0/72** |

OECD Family Database: only PDF documents are served on webfs (for example `SF_1_1_Family_size_and_composition.pdf`).
No SDMX dataflow exists for it, and I found no machine-readable file. IPUMS International needs a login.

## 3. Housing tenure

| Dataset / publisher | URL | Format | Measures | Coverage 2010+ |
|---|---|---|---|---|
| **OECD Affordable Housing Database, HM1.3 Housing tenures** (OECD 2025, "https://oe.cd/ahd") | `https://webfs.oecd.org/Els-com/Affordable_Housing_Database/HM1-3-Housing-tenures.xlsx` | xlsx, several sheets | **HM1.3.A1**: % of households by tenure, by year 2010–2024: own outright; owner with mortgage; rent (private); rent (subsidised); other/unknown. A2 gives the same as % of population. HM1.3.4/A6 break tenure down by age; HM1.3.2/A3/A4 by income quintile. Sources: EU-SILC, HILDA, CIS, CASEN, GEIH, ENAHO, ENIGH, ACS, and others. Private and subsidised renting are combined for AUS, AUT, CAN, CHL, COL, CRI, DNK, MEX, NZL, TUR and USA. Outright and mortgaged owners are combined for KOR and TUR. | **38/86, 3/59 (COL, MEX, TUR), 0/72** |
| Eurostat `ilc_lvho02` "Distribution of population by tenure status, type of household and income group" | `…/sdmx/2.1/data/ilc_lvho02/?format=SDMX-CSV&startPeriod=2010` | % of **persons** | OWN, OWN_L (with mortgage or loan), OWN_NL, RENT_MKT, RENT_FR (reduced or free) | **31/86, 6/59, 0/72** |
| **CEPALSTAT indicator 166** "Households, by housing tenure status and area" (ECLAC, BADEHOG household surveys) | `https://api-cepalstat.cepal.org/cepalstat/api/v1/indicator/166/data?lang=en&format=json` (dimension labels at `/indicator/166/dimensions`) | JSON, ids decoded through the dimensions call | % of households: owner / tenant / other forms; national, urban and rural. No split between outright owners and owners with a mortgage. | **4/86, 9/59, 4/72** (BOL, BRA, CHL, COL, CRI, DOM, ECU, GTM, HND, MEX, NIC, PAN, PER, PRY, SLV, URY, VEN) |
| CEPALSTAT 4619 "Population, by housing tenure status, quintiles, and area" | same API, id 4619 | JSON | Same, by persons and income quintile | not counted |
| UNSD Demographic Yearbook census table 304 "Households in housing units by type of housing unit, tenure of household and urban/rural residence" | page: `https://data.un.org/Data.aspx?d=POP&f=tableCode%3a304` | HTML only. The CSV handler did not work from here. | Census households by tenure | 142 areas have **some** year (any year, not only 2010+): 61/86, 35/59, 33/72. A 2010+ count was not possible without the data. |
| DHS API `WE_OWNA_W_HAJ` / `WE_OWNA_M_HAJ` | DHS API as in need 2 | JSON | **Individuals** (women or men 15–49) who own a house alone and/or jointly. This is not household tenure. | W: **0, 10, 47**; M: **0, 8, 43** |

The DHS API has **no household dwelling-tenure indicator**. Its HC_ family covers floors, rooms, fuel and assets only.

## 4. Mortgages, deposits and accounts

| Dataset / publisher | URL | Format | Measures | Coverage 2010+ |
|---|---|---|---|---|
| OECD AHD HM1.3 (as in need 3) | as above | xlsx | **Share of households that are "owner with mortgage"**, by year, age and quintile | 38 / 3 / 0 |
| OECD AHD HC1.2 Housing costs over income | `https://webfs.oecd.org/Els-com/Affordable_Housing_Database/HC1-2-Housing-costs-over-income.xlsx` | xlsx | Median mortgage burden (principal + interest) as % of disposable income, for owners with a mortgage (HC1.2.1, A1_a by year); overburden rates | 38 / 3 / 0 |
| OECD Wealth Distribution Database `OECD.WISE.INE,DSD_WEALTH@DF_WEALTH` | `https://sdmx.oecd.org/public/rest/data/OECD.WISE.INE,DSD_WEALTH@DF_WEALTH,/all?format=csvfilewithlabels` | CSV | Share of indebted households, debt-to-income and debt-to-assets of indebted households, top 1/5/10% and bottom 40% wealth shares. No mortgage-specific series. | **31/86, 0/59, 0/72** |
| ECB MFI interest rate statistics (MIR), new business lending for house purchase | `https://data-api.ecb.europa.eu/service/data/MIR/M..B.A2C.A.R.A.2250.EUR.N?startPeriod=2024-01&format=csvdata` | CSV | Rate on new loans to households for house purchase, % a year, by euro-area country | **21/86, 0, 0** |
| **World Bank Global Findex** (WB API source 28, updated 2025-10-06; waves 2011, 2014, 2017, 2021, 2022, 2024) | `https://api.worldbank.org/v2/country/all/indicator/account.t.d?format=json&date=2010:2025&per_page=20000&source=28`. `countryiso3code` is blank here; use `country.id`. | JSON | `account.t.d` (account, % age 15+), `fiaccount.t.d`, `fin17a` (saved at a financial institution), `fin17f` (saved for old age, 2024 only), `fin22a` (borrowed from a formal institution), `borrow.any.t.d`, `fin38` (received a public-sector pension) | account / fin17a: **53/86, 45/59, 63/72**; fin22a: 52/45/61; fin38: 50/45/56; fin17f: 8/41/49 |
| Global Findex microdata (catalogue) | `https://microdata.worldbank.org/index.php/api/catalog/search?sk=findex` (metadata 200) | – | Individual responses | Download not tested; the WB microdata library normally needs registration |
| World Bank GFDD `GFDD.AI.04` "Small firms with a bank loan or line of credit (%)" | WB API source 32 | JSON | (firms, not households) | 38 / 42 / 60 |

**No machine-readable source** was found for: home-loan borrowing by households in emerging or developing economies
(the current Findex API carries no "borrowed for a home" series); a typical mortgage rate outside the euro area; or
**remaining mortgage term** in any group. The ECB HFCS has these, but its microdata need an application and its
tables are PDF. HOFINET (`hofinet.org`) answers, but its data are interactive pages and I found no download.

## 5. Education attainment by age; occupation; status in employment

| Dataset / publisher | URL | Format | Measures | Coverage 2010+ |
|---|---|---|---|---|
| **Wittgenstein Centre Human Capital Data Explorer v3 (2023)**, SSP2 (scenario 2), batch file | `https://wicshiny2023.iiasa.ac.at/wcde-data/wcde-v3-batch/2/prop.rds` (7.5 MB; also `bprop.rds`, and later versions under `wcde-v31-`, `wcde-v32-`) | R .rds; `country_code` (M49), `year` (1950–2100, 5-yearly), `age` (15–19…100+), `sex`, `education` (9 levels: none, incomplete primary, primary, lower secondary, upper secondary, post-secondary, short post-secondary, bachelor, master+), `prop` (%) | Attainment distribution by 5-year age group and sex. It is an estimate and projection, based on censuses and surveys, with no gaps. | 2020 rows: **66/86, 55/59, 72/72** |
| **Barro-Lee v3 (2021)**, GitHub Pages mirror | `https://barrolee.github.io/BarroLeeDataSet/BLData/BL_v3_MF.csv` (also `BL_v3_M.csv`, `BL_v3_F.csv`, `BL_v3_MF1564.csv`). `barrolee.com` returns 302. | CSV; `WBcode, year, agefrom, ageto, lu, lp, lpc, ls, lsc, lh, lhc, yr_sch…` | % of the age group with no schooling, primary (total and completed), secondary, and tertiary, plus mean years. **10-year age groups 15–64 only; years to 2015.** | **55/86, 38/59, 49/72** |
| Barro-Lee v2.2 (2013) | `https://barrolee.github.io/BarroLeeDataSet/BLData/BL2013_MF_v2.2.csv` | CSV | 5-year groups 15–19…75+; **last year 2010** | 2010: 55 / 38 / 49 |
| UNESCO UIS API | `https://api.uis.unesco.org/api/public/data/indicators?indicator=EA.3T8.AG25T99&start=2010` | JSON | Attainment rate by ISCED level, **population 25+ only**. There are no age-group series (all 271 EA.* codes are AG25T99). | EA.3T8: 69/86, 54/59, 66/72 |
| Eurostat `lfsa_egised` "Employed persons by occupation and educational attainment level" | Eurostat SDMX | CSV, thousands | ISCO-08 × ISCED | 31 / 5 / 0 |
| **ILOSTAT `DF_EMP_TEMP_SEX_OCU_NB`** "Employment by sex and occupation" | `https://sdmx.ilo.org/rest/data/ILO,DF_EMP_TEMP_SEX_OCU_NB,1.0/.A..SEX_T.?startPeriod=2010` | SDMX-CSV; thousands; `OCU_ISCO08_0…9, X, TOTAL`, also ISCO-88 and skill levels; `SOURCE` (mostly LFS) | Employed persons by ISCO major group | ISCO-08 group 1: **59/86, 51/59, 62/72** (none from modelled estimates); skill level: 65/53/64 |
| ILOSTAT `DF_EMP_TEMP_SEX_AGE_OCU_NB`, `DF_EMP_TEMP_SEX_OCU_EDU_NB` | same pattern | – | Occupation by age; occupation by education | listed, not counted. `DF_EMP_TEMP_SEX_AGE_EDU_NB` timed out (504) with key `all`; it needs a narrower key. |
| **ILOSTAT `DF_EMP_TEMP_SEX_STE_NB`** "Employment by sex and status in employment" | `https://sdmx.ilo.org/rest/data/ILO,DF_EMP_TEMP_SEX_STE_NB,1.0/all?startPeriod=2010` (48 MB) | SDMX-CSV; `STE_ICSE93_1` employees, `_2` employers, `_3` own-account workers, `_4` members of producers' cooperatives, `_5` contributing family workers, `_6` not classifiable; aggregates EES/SLF | Employed persons by ICSE-93 status | own-account, survey sources only: **68/86, 53/59, 66/72**; employers 66/51/66. The file also holds ILO modelled estimates, which fill the rest. |
| DHS `EM_OCCP_W_*` / `EM_OCCP_M_*` | DHS API | JSON | Coarse occupation groups for women and men aged 15–49 | men: 0 / 9 / 44 |

## 6. Income and wealth percentile shares (WID)

| Dataset | URL | Format | Measures | Coverage 2010+ |
|---|---|---|---|---|
| World Inequality Database bulk (last-modified 2026-09-09, 882 MB) | `https://wid.world/bulk_download/wid_all_data.zip` | zip of `WID_data_XX.csv` (`;`-separated: country, variable, percentile, year, value) and `WID_metadata_XX.csv` | Variable names use the form `<type><concept><pop><age>`: **`sptincj992`** = share of pre-tax national income, equal-split adults 20+; **`shwealj992`** = share of net personal wealth; also `aptincj992` / `ahwealj992` (averages), `tptincj992` / `thwealj992` (thresholds), `sdiincj992` (post-tax disposable) | For **sptincj992 and shwealj992**, every one of p0p10…p90p100, p99p100 and p99.9p100 is present in 2010+ for **79/86, 58/59, 72/72**. In every one of those economies at least one year has all ten deciles and at least 127 g-percentiles (the full p0p1…p99.999p100 grid). Last year: 2024. |

**Quality caveat, from the `avg_quality` field of the WID metadata files:**
- `shwealj992` has avg_quality 0.0 in 47/79 dev, 55/58 emg and 71/72 dvp.
- `sptincj992` has avg_quality 0.0 in 27/79 dev, 17/58 emg and 11/72 dvp.

The wealth shares for emerging and developing economies therefore come almost entirely from WID's imputation, not
from country source data. Income shares are much better grounded. fetch.py's existing names (`shwealj992`,
`whweali999`) follow the same convention. Note the variable name is `sptincj992`, not `sptinc992j`.

## 7. Pensions

| Dataset / publisher | URL | Format | Measures | Coverage 2010+ |
|---|---|---|---|---|
| **OECD Pensions at a Glance** `OECD.ELS.SPD,DSD_PAG@DF_PAG` (2025 edition data, 2024 values) | `https://sdmx.oecd.org/public/rest/data/OECD.ELS.SPD,DSD_PAG@DF_PAG,/all?format=csvfilewithlabels` | CSV | `CRPLF22` current retirement age (entry at 22); `FRPLF22` future retirement age; `GPRR/NPRR 50/100/200` gross and net replacement rates; `GPW/NPW` pension wealth; `ELMEA` effective exit age; `PTOP`/`OCOP`/`CIOP`/`EIOP` public transfers, occupational transfers, capital income and work income of older people; `PEP` public pension spending | Retirement age and replacement rates: **41/86, 8/59 (ARG, BRA, CHN, COL, IDN, MEX, TUR, ZAF), 1/72 (IND)**. Income-source measures: 35/3/0. |
| OECD `DSD_PAG@DF_DPS` "Design of pension systems" | same agency | CSV | Only CRPLF22 and FRPLF22 | 41–42 / 8 / 1 |
| **ILOSTAT SDG 1.3.1 `DF_SDG_0131_SEX_SOC_RT`** | `https://sdmx.ilo.org/rest/data/ILO,DF_SDG_0131_SEX_SOC_RT,1.0/all?startPeriod=2010` | SDMX-CSV, % | `SOC_CONTIG_PENSION` = share of the population above statutory pensionable age receiving an old-age pension (also disability, unemployment, child and other functions). SOURCE is "ILO – Social Security Inquiry Database" or "ILO – Modelled Estimates". | pension: **77/86, 58/59, 69/72**. The same count holds with modelled rows excluded. |
| **OECD Global Pension Statistics** (asset-backed pensions) `OECD.DAF.CM,DSD_FP@DF_FPS` (main, 423 MB since 2010), `@DF_SPS` (structure), `@DF_MB` (membership), `@DF_BC` (benefits and contributions), `@DF_PA` | `https://sdmx.oecd.org/public/rest/data/OECD.DAF.CM,DSD_FP@DF_SPS,/all?startPeriod=2010&format=csvfilewithlabels` | CSV; dims PLAN_TYPE (OCC/PER), DEFINITION_TYPE (DB, DCU, …), VEHICLE_TYPE | Pension-fund investments by asset class; **DB pension assets (`DBA`)**; DC assets; occupational and personal assets; active and passive members; benefits paid and contributions (DF_BC) | total investment: **49/86, 26/59, 19/72**; DB assets (`DBA`, OCC, DB): **24/86, 5/59, 9/72**; active members: 41/22/16 |
| World Bank GFDD `GFDD.DI.13` pension fund assets to GDP | WB API source 32 | JSON | % of GDP | 48 / 24 / 19 |
| OECD Taxing Wages SSC rates `OECD.CTP.TPS,DSD_TAX_SSC@DF_SSC_EMPLOYEE / _EMPLOYER / _SELF` | sdmx.oecd.org (listed, not downloaded) | CSV | Statutory social-security contribution rates | OECD members only |
| CEPALSTAT 3136 / 3137 / 3139 | CEPALSTAT API | JSON | Employed and wage-earning population contributing to or enrolled in a pension system | Latin America only (not counted) |

**Not reachable**: SSA/ISSA "Social Security Programs Throughout the World" (403) and the ILO World Social Protection
Database (403). With those blocked, **no downloadable source was found for statutory pension age, pension rules or
mandatory contribution rates outside OECD PAG's 50 economies.** The WB API has only the WBL yes/no items, for
example "the ages at which a woman and a man can retire are the same", and no ages.

## 8. Illness and disability

| Dataset / publisher | URL | Format | Measures | Coverage 2010+ |
|---|---|---|---|---|
| Eurostat `hlth_silc_06` "Self-perceived long-standing limitations in usual activities due to health problem by sex, age and labour status" (91 MB) | Eurostat SDMX, as above | CSV, % | GALI: severe / some / none; ages 16–24 … 85+ in about 10-year bands; by labour status (employed, unemployed, retired, other inactive) | **31/86, 6/59, 0/72** |
| Eurostat `hlth_silc_07` / `hlth_silc_12` "Level of disability (activity limitation) by sex, age and education / income quintile" | same | % | same, by education or quintile | 31 / 6 / 0 |
| **ILOSTAT `DF_POP_XWAP_SEX_AGE_DSB_NB`** "Working-age population by sex, age and disability status" (31 MB) | `https://sdmx.ilo.org/rest/data/ILO,DF_POP_XWAP_SEX_AGE_DSB_NB,1.0/all?startPeriod=2010` | SDMX-CSV, thousands; `DSB_STATUS_DIS/NODIS`; ages 15–24, 25–54, 55–64, 65+ | Persons with disability, from LFS, EU-SILC and household surveys (per-source definitions, mostly Washington Group) | **41/86, 26/59, 51/72** |
| ILOSTAT `DF_EAP_DWAP_SEX_AGE_DSB_RT` and related `*_DSB_*` flows | same | – | Participation and employment by disability status | listed, not counted |
| **World Bank Disability Data Hub** (WB API source 92), e.g. `adj_al_alot_dfcl_4554`, `…_65up` | `https://api.worldbank.org/v2/country/all/indicator/adj_al_alot_dfcl_4554?format=json&date=2000:2025&per_page=20000&source=92` | JSON | Adjusted prevalence of persons with "at least a lot of functional difficulty" (Washington Group), by age 15–24 … 65+, and by domain | **2/86, 15/59, 43/72** |
| ILOSTAT SDG 1.3.1 `SOC_CONTIG_DISAB` | as need 7 | % | Share of severely disabled persons receiving a disability benefit | 75 / 55 / 54 |

IHME GBD: the results tool (`vizhub.healthdata.org/gbd-results`) answers 200, but downloads need a registered login,
so I did not use it. The DHS API has no disability indicator.

**No source of incidence (onset rates)** of work-limiting illness was found for any group. Everything above measures
prevalence.

## 9. Kin: adult children living in other households

| Dataset | URL | Measures | Coverage 2010+ |
|---|---|---|---|
| UN Living Arrangements of Older Persons 2026 (need 2) | as above | % of persons aged 60+ / 65+ / 80+ **co-residing** with a child aged 20+ | 16 / 48 / 63 |
| Eurostat `ilc_lvps08` "Persons living with their parents… (population aged 18 to 34 years)" | Eurostat SDMX | % of 18–34-year-olds still living with a parent, by age band and sex | 31 / 6 / 0 |
| Eurostat `yth_demo_030` "Estimated average age of young persons leaving the parental household" | Eurostat SDMX | years | 29 / 5 / 0 |

**None found** that counts adult children living *outside* the parent's household by parent's age. SHARE, HRS,
CHARLS and similar surveys have this, but they need registration. The published sources give only the co-resident
side.

## 10. Job tenure

| Dataset / publisher | URL | Format | Measures | Coverage 2010+ |
|---|---|---|---|---|
| **OECD `OECD.ELS.SAE,DSD_TENURE@DF_TENURE_FREQ`** "Employment by job tenure intervals – frequency" (178 MB with labels since 2010) | `https://sdmx.oecd.org/public/rest/data/OECD.ELS.SAE,DSD_TENURE@DF_TENURE_FREQ,/all?startPeriod=2010&format=csvfilewithlabels` | CSV | % of employed (`TENURE_EMP`) or employees (`TENURE_EMP_DEP`) by tenure with the current employer: <1 m, 1–<6 m, 6–<12 m, 1–<3 y, 3–<5 y, 5–<10 y, 10 y+; by 5-year age groups and sex | **39/86, 4/59 (BRA, COL, MEX, TUR), 0/72** |
| OECD `@DF_TENURE_AVE` (average tenure, years) and `@DF_TENURE_DIS` (persons) | same | CSV | Mean tenure | AVE: 32 / 2 / 0 |

ILOSTAT has no job-tenure dataflow among its 1,214 SDMX dataflows. DHS has none either.

## 11. Rents

| Dataset | URL | Measures | Coverage 2010+ |
|---|---|---|---|
| OECD AHD HC1.2 (need 4) | webfs xlsx | Median rent burden (private and subsidised rent) as % of disposable income for tenant households, by year (HC1.2.A1_a) and by quintile; overburden rates (>40%) | 38 / 3 / 0 |
| Eurostat `ilc_lvho07a` "Housing cost overburden rate by age, sex and poverty status" | Eurostat SDMX | % of persons whose total housing cost exceeds 40% of disposable income | 31 / 6 / 0 |

**No machine-readable rent-level or rent-burden source** was found for emerging or developing economies beyond COL,
MEX and TUR in the AHD and the Eurostat candidate and neighbour countries.

## 12. Small firms below the top

| Dataset / publisher | URL | Format | Measures | Coverage 2010+ (2015+ for SDBS) |
|---|---|---|---|---|
| OECD SDBS `OECD.SDD.TPS,DSD_SDBSBSC_ISIC4@DF_SDBS_ISIC4`, all size classes | `https://sdmx.oecd.org/public/rest/data/OECD.SDD.TPS,DSD_SDBSBSC_ISIC4@DF_SDBS_ISIC4,/A..ENTR+EMPN.BTN_95XK..?startPeriod=2015&format=csvfile` (fetch.py's URL with the size filter removed) | CSV | Enterprises and persons employed by size class: 1–9, 10–19, 20–49, 50–249, 250+ (and 1–19, 10–49, 1–249, ≥10); business economy B–N and S95, excluding K. No 1-person class in this query. | **30–31/86, 4/59 (ALB, BIH, MKD, SRB), 0/72** |
| **ILOSTAT `DF_EMP_TEMP_SEX_EST_NB`** "Employment by sex and establishment size" (25 MB) | `https://sdmx.ilo.org/rest/data/ILO,DF_EMP_TEMP_SEX_EST_NB,1.0/all?startPeriod=2010` | SDMX-CSV, thousands | **Employed persons (formal and informal) by size of the establishment they work in**: 1, 2–4, 5–9, 10–19, 20–49, 50+ (aggregates 1–4, 5–49, 50+). From labour force and household surveys. | 1–4: **20/86, 36/59, 54/72**; single person (S1): 17 / 27 / 48 |
| ILOSTAT `DF_EMP_TEMP_SEX_STE_EST_NB` (status × establishment size), `DF_EES_TEES_SEX_EST_NB` (employees only), `DF_EMP_NIFL_SEX_EST_*` (informal) | same | – | Own-account and employers by establishment size; informality | listed, not counted |
| World Bank Enterprise Surveys | API source 13 has 115 aggregate indicators and **no size distribution**. Microdata sit behind `login.enterprisesurveys.org`. The IFC MSME Country Indicators pages return 404. | – | The ES samples registered firms with 5+ employees, so it cannot give the population of firms by size in any case | – |
| Eurostat `sbs_sc_ovw` "Enterprise statistics by size class and NACE Rev. 2 activity (from 2021 onwards)" | JSON API works with filters; the full SDMX-CSV request is refused as too large (413) | – | EU enterprises by size, including 0–1 persons | EU/EFTA (not counted) |

---

## Gaps, per need, and the nearest defensible proxy

1. **Population and life tables.** No gap. WPP 2024 covers every group completely.
2. **Household composition.**
   - Emerging and developing are well covered: UN DESA HH 2026 at 48–49/59 and 63/72.
   - **Developed is thin on the detailed types.** Extended, non-relative and multi-generation households are known
     for only 17–18/86 economies, and older persons with adult children for 16/86.
   - Proxy for developed: Eurostat `ilc_lvph02`/`lfst_hhnhtych` (31/86) for the basic types, where `A_GE3` (3+ adults)
     is the nearest stand-in for extended. The gap: `A_GE3` does not separate relatives from non-relatives or count
     generations.
3. **Tenure.**
   - **Developing: no source splits outright owners, mortgaged owners and private or social renters.** CEPALSTAT 166
     gives owner / tenant / other for only 4/72 (BOL, HND, NIC, VEN).
   - Emerging: CEPALSTAT 166 covers 9/59 and the AHD 3/59, with no mortgage split in CEPALSTAT.
   - Nearest proxy: DHS `WE_OWNA_*_HAJ` (individuals aged 15–49 owning a house alone or jointly; 47/72 dvp, 10/59
     emg). The gap: it counts persons, not households; it covers ages 15–49 only; and it includes owning any house,
     not only the dwelling occupied.
   - The UNSD DYB census table 304 has national tenure for 33 developing and 35 emerging economies in some year, but
     it could not be downloaded here.
4. **Mortgages.**
   - Share with a mortgage exists for **developed only** (AHD, 38/86).
   - **Emerging and developing: none.**
   - Nearest proxy: Findex `fin22a` (borrowed from a formal financial institution in the past year, % 15+). The gap:
     it covers any purpose, it is a flow rather than an outstanding stock, and it counts persons.
   - Mortgage rate: euro area only (ECB MIR, 21/86). For other economies the nearest proxy is WDI `FR.INR.LEND`,
     which is already fetched. The gap: it is the prime or short-term business lending rate, not a housing rate.
   - **Remaining term: no source in any group.**
   - Accounts and deposits: Findex covers all groups (53/45/63).
5. **Education, occupation and status in employment.**
   - No group gap: Wittgenstein v3 gives attainment by 5-year age group for 66/55/72, and ILOSTAT occupation and
     status data cover 59–68/51–54/62–66.
   - Barro-Lee is older: it stops in 2010 (5-year groups, v2.2) or 2015 (10-year groups up to 64, v3).
6. **WID.** Series exist for every group (79/58/72), but **wealth shares are imputed** for 55/58 emerging and 71/72
   developing economies (avg_quality 0). No proxy exists that would be better than WID's own imputation. This is
   stated here, not fixed.
7. **Pensions.**
   - **Statutory pension age, pension rules and contribution rates are missing for emerging (51/59) and developing
     (71/72) economies.** The only reachable source is OECD PAG (8 emerging, 1 developing). SSA/ISSA and the ILO
     WSPDB are blocked (403).
   - Recipients (SDG 1.3.1) cover all groups.
   - DB assets: 24/5/9. Pension-fund totals: 49/26/19.
   - The nearest defensible proxy for developing pension age is none from a dataset reachable here. Retrieving ISSA
     or SSPTW from another network is the fix; the owner decides.
8. **Illness and disability.**
   - Prevalence is covered by ILOSTAT DSB (41/26/51) and WB DDH (2/15/43), with Eurostat GALI for developed (31).
   - **Incidence or onset: no source in any group.**
   - Definition gap: Washington Group "functional difficulty" (ILO, WB) versus GALI "limitation in usual activities"
     (EU-SILC). Neither is "work-limiting" as such, except hlth_silc_06 read together with labour status.
9. **Kin.** **None** in any group for adult children living elsewhere. Only the co-resident side is published: UN
   (16/48/63) and Eurostat (31/6/0).
10. **Job tenure.** Developed 39/86 (OECD). **Emerging: only BRA, COL, MEX, TUR. Developing: none.** No defensible
    proxy exists in the reachable data.
11. **Rents.** Developed only (AHD 38, Eurostat 31); emerging 3–6; **developing none.**
12. **Small firms.**
    - Enterprise counts by size are known for developed (30–31/86) and 4 emerging economies (SDBS).
    - For emerging and developing, the nearest proxy is **ILOSTAT employment by establishment size** (36/59, 54/72;
      and 20/86 developed). The gap: it counts persons employed by the size of their workplace (an establishment,
      including informal and agricultural work), not enterprises. Enterprise counts per class would have to be
      inferred by dividing by class-average size, which the data do not give.
