import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const root = new URL("..", import.meta.url).pathname;
const source = join(root, "packages/kernel-rs/src");
const forbidden = [
  [/\bDay\s*\(/, "simulation Day literal"],
  [/Convention::Actual(?:360|365)/, "day-count convention"],
  [/Dimension::(?:Days|Months|Years)/, "non-week time dimension"],
  [/\.\s*(?:days|months|years)\s*\(/, "non-week parameter read"],
  [/\b(?:weekday|is_a_business_day|nth_weekday_of_its_month|last_business_day_of_its_month|plus_months)\s*\(/, "civil-calendar operation"],
  [/[\/]\s*(?:360(?:\.0)?|365(?:\.0)?)(?![\d])/, "literal day-basis division"],
];

function files(at) {
  return readdirSync(at).flatMap((name) => {
    const path = join(at, name);
    return statSync(path).isDirectory() ? files(path) : path.endsWith(".rs") ? [path] : [];
  });
}

const failures = [];
for (const path of files(source)) {
  const lines = readFileSync(path, "utf8").split("\n");
  lines.forEach((line, index) => {
    if (line.trimStart().startsWith("//")) return;
    for (const [pattern, description] of forbidden) {
      if (pattern.test(line)) failures.push(`${relative(root, path)}:${index + 1}: ${description}`);
    }
  });
}

if (failures.length) {
  console.error("Weekly calendar boundary violations:\n" + failures.join("\n"));
  process.exitCode = 1;
} else {
  console.log("Weekly calendar boundary check passed");
}
