var sidebar_hidden = false;

var make_note_entry = function (file, title, tags) {
    var entry = $(
        "<div class='search-result'>" +
        "<div class='search-result-tags'>" + tags.join(", ") + "</div>" +
        "<div class='search-result-title'>" + title + "</div>" +
        "</div>"
    );

    $(entry).click(function () {
        render_file(file);
    });

    $("#search-results").append(entry);
}

var render_file = function (file) {
    $.post("/notes/" + file, function (content) {
        // don't do anything if we get an empty response
        if (!content) return;

        content = content.replace("<head>", "<head><base target=\"_blank\">");
        $("#content-html").attr("srcdoc", content);

        // if we're on mobile, hide the sidebar
        const is_mobile = window.matchMedia("only screen and (max-width: 760px)").matches;
        if (is_mobile) {
            toggle_sidebar();
        }

        // fix page links
        setTimeout(function () {
            $("#content-html").contents().find("body").delegate("a", "click", function (e) {
                var link_id = $(this).attr("href");
                if (link_id.startsWith("#")) {
                    e.preventDefault();
                    $("#content-html").contents().find(link_id).get(0).scrollIntoView();
                }
            });
        }, 100);
    });
}

var do_search = function (term) {
    // if the search term is empty, do a search for everything
    if (term == "") term = "*";

    $.ajax("/search", {
        type: "POST",
        data: JSON.stringify({ "search_term": term }),
        contentType: "application/json",
        success: function (data) {
            if ("responses" in data && data["responses"].length > 0) {
                // remove all note entries in list
                $("#search-results").empty();

                // add one entry for each response
                data["responses"].forEach(note => {
                    make_note_entry(note.file, note.title, note.tags);
                });

                $("#search").css("border-color", "white");
            } else {
                $("#search").css("border-color", "#ff4a4a");
            }
        }
    });
}

var toggle_sidebar = function () {
    if (sidebar_hidden) {
        // reveal sidebar
        $("#sidebar").removeClass("hidden");
        $("#burger").removeClass("hidden");
        $("#content").removeClass("hidden");
    } else {
        // hide sidebar
        $("#sidebar").addClass("hidden");
        $("#burger").addClass("hidden");
        $("#content").addClass("hidden");
    }

    sidebar_hidden = !sidebar_hidden;
}

$(function () {
    $("#search").on("input", function () {
        do_search($("#search").val());
    });

    $("#burger").click(function () {
        toggle_sidebar();
    });

    $("#sidebar-title > span").click(function () {
        $("#search").val("");
        do_search("");
    });

    do_search("");
});