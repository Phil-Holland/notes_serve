const mume = require("@shd101wyy/mume");
const fs = require("fs");
const path = require("path");

module.exports = {
    render_file: async function (filepath) {
        await mume.init();

        const engine = new mume.MarkdownEngine({
            filePath: filepath,
            projectDirectoryPath: "./output",
            config: {
                previewTheme: "github-light.css",
                codeBlockTheme: "darcula.css",
                printBackground: true,
                enableScriptExecution: false,
            },
        });

        // html export
        const outputFilepath = await engine.htmlExport({ offline: false, runAllCodeChunks: true });
        const finalOutputFilepath = "./output/" + path.basename(outputFilepath);

        // move HTML to output folder
        fs.renameSync(outputFilepath, finalOutputFilepath);

        console.log("Rendering completed - output HTML: " + finalOutputFilepath);

        return finalOutputFilepath;
    }
}