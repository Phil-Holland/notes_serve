var make_note_entry = function (file, title, tags) {
    var entry = $(
        "<div class='search-result' onclick='render_file(\"" + file + "\")'>" +
        "<div class='search-result-tags'>" + tags.join(", ") + "</div>" +
        "<div class='search-result-title'>" + title + "</div>" +
        "</div>"
    );

    $("#search-results").append(entry);
}

var render_file = function (file) {
    $.post("/notes/" + file, function (content) {
        if (!content) return;

        content = content.replace("<head>", "<head><base target=\"_blank\">");
        $("#content-html").attr("srcdoc", content);

        // fix page links
        setTimeout(function () {
            $("#content-html").contents().find("body").delegate("a", "click", function (e) {
                var link_id = $(this).attr("href");
                if (link_id.startsWith("#")) {
                    e.preventDefault();
                    $("#content-html").contents().find(link_id).get(0).scrollIntoView();
                }
            });
        }, 10);
    });
}

var do_search = function (term) {
    if (term == "") term = "*";

    $.post("/search", term, function (data) {
        if ("responses" in data && data["responses"].length > 0) {
            // remove all note entries in list
            $("#search-results").empty();

            data["responses"].forEach(note => {
                make_note_entry(note.file, note.title, note.tags);
            });
        }
    });
}

$(function () {
    $("#search").on("input", function () {
        do_search($("#search").val());
    });

    do_search("");
});