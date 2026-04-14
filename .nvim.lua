vim.api.nvim_create_autocmd("BufWritePost", {
    pattern = "*.c,*.s",
    command = "silent make | cwindow"
})

vim.lsp.config().clangd.setup({
    cmd = {
        "clangd",
        "--query-driver=i686-elf-gcc",
        "--background-index",
        "--clang-tidy",
    },
})

vim.o.tabstop = 8
vim.o.shiftwidth = 8
