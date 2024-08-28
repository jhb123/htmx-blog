function initializeDialog() {
    var dialog = document.getElementById("adminDialog");
    var showButton = document.getElementById("showDialog");
    var closeButton = document.getElementById("closeAdminDialog");
    var adminLogin = document.getElementById("page-content");

    if (showButton) {
        showButton.addEventListener("click", () => {
            dialog.showModal();
        });
    }

    if (closeButton) {
        closeButton.addEventListener("click", () => {
            dialog.close();
        });
    }

    if (adminLogin) {
        adminLogin.addEventListener('htmx:afterSwap', function(evt) {
            console.log(evt.detail.xhr.status)
            if (evt.detail.xhr.status === 200) {
                dialog.close();
            }
        });
    }
}

document.addEventListener('DOMContentLoaded', function() {
    initializeDialog();
});

document.addEventListener('htmx:afterSwap', function(event) {
    initializeDialog();
});