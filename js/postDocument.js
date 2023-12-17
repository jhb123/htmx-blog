var adminFormContainer = document.getElementById("admin-form-container");

adminFormContainer.addEventListener('htmx:configRequest', function(evt) {
    console.log(evt.detail)
    let files = evt.detail.parameters.files
    console.log(files)
    let filesArray = Array.from(files);
    let newFilesArray = []

    filesArray.forEach((file, index) => {
                
        if (file.name.substring(file.name.length - 3) === '.md') {
            let name = file.name
            console.log(name)

            const blobOptions = { type: "text/markdown" };
            const blob = new Blob([file.slice(0, file.size)], blobOptions);

            newFilesArray.push(blob)
        } else {
            newFilesArray.push(file)
        }
    });

    evt.detail.parameters.files = newFilesArray
    console.log(evt.detail.parameters.files)

});