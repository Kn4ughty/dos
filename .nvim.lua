vim.api.nvim_create_autocmd("BufWritePost", {
    pattern = "*.c,*.s",
    command = "silent make | cwindow"
})
