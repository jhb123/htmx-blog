var dialog = document.getElementById("adminDialog");
var showButton = document.getElementById("showDialog")
var closeButton = document.getElementById("closeAdminDialog");

// "Show the dialog" button opens the dialog modally
showButton.addEventListener("click", () => {
    dialog.showModal();
});

// "Close" button closes the dialog
closeButton.addEventListener("click", () => {
    dialog.close();
});