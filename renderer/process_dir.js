const fs = require("fs");
const path = require("path");
const YAML = require("yaml");
const removeMd = require("remove-markdown");
const { render_file } = require("./render_file");

async function main() {
    const args = process.argv;

    if (args.length != 3) {
        console.error("ERROR: not enough arguments supplied.\nUsage: node process_dir.js [/path/to/notes_directory]")
        return process.exit(1);
    }
    const inputDir = args[2];

    var summary = [];

    const dir = fs.opendirSync(inputDir)
    var dirent;
    while ((dirent = dir.readSync()) !== null) {
        const ext = path.extname(dirent.name);
        if (ext === ".md") {
            const fullPath = inputDir + "/" + dirent.name;
            try {
                // get file contents & metadata
                const fileContentsSplit = fs.readFileSync(fullPath, "utf8").split("---");

                var content = "";
                var title = path.parse(dirent.name).name;
                var tags = [];
                if (fileContentsSplit.length == 1) {
                    content = removeMd(fileContentsSplit[0]);
                } else if (fileContentsSplit.length == 2) {
                    content = removeMd(fileContentsSplit[1]);
                } else {
                    content = removeMd(fileContentsSplit.slice(2).join("---"));

                    var parsedMetadata = YAML.parse(fileContentsSplit[1]);
                    if ("title" in parsedMetadata && typeof (parsedMetadata["title"]) === "string") {
                        title = parsedMetadata["title"];
                    }

                    if ("tags" in parsedMetadata && Array.isArray(parsedMetadata["tags"])) {
                        tags = parsedMetadata["tags"];
                    }
                }


                // render HTML for this file
                const renderedFilepath = await render_file(fullPath);

                summary.push({
                    "file": path.basename(renderedFilepath),
                    "title": title,
                    "tags": tags,
                    "content": content
                });
            } catch (e) {
                console.warn("Could not process file: " + fullPath);
                console.error(e);
            }
        }
    }
    dir.closeSync()

    // write output summary JSON file
    fs.writeFileSync("./output/summary.json", JSON.stringify(summary));

    return process.exit();
}

main();