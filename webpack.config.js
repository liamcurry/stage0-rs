const path = require("path");
const HtmlWebpackPlugin = require("html-webpack-plugin");
const CleanWebpackPlugin = require("clean-webpack-plugin");
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");
const MiniCssExtractPlugin = require("mini-css-extract-plugin");

module.exports = ["todomvc"].map(pkg => {
    const crateDirectory = path.resolve(__dirname, "examples", pkg);
    const outPath = path.resolve(__dirname, "gh-pages", pkg);
    return {
        mode: "production",
        entry: path.resolve(crateDirectory, "static", "index.js"),
        output: {
            path: outPath,
            filename: "index.js",
            publicPath: `/${pkg}/`,
        },
        devServer: {
            contentBase: path.resolve(__dirname, "gh-pages"),
        },
        module: {
            rules: [
                {
                    test: /\.css$/,
                    use: [
                        "style-loader",
                        MiniCssExtractPlugin.loader,
                        "css-loader",
                    ],
                },
            ],
        },
        plugins: [
            new CleanWebpackPlugin({
                dry: true,
                cleanOnceBeforeBuildPatterns: [
                    path.resolve(crateDirectory, "pkg", "*"),
                ],
            }),
            new MiniCssExtractPlugin({
                filename: "index.css",
            }),
            new WasmPackPlugin({
                crateDirectory,
                watchDirectories: [path.resolve(__dirname, "src")],
            }),
            new HtmlWebpackPlugin({
                template: path.resolve(crateDirectory, "static", "index.html"),
                minify: true,
            }),
        ],
    };
});
