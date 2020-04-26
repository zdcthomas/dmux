args="$@"
version=${args[0]}
targets=("x86_64-apple-darwin")
files=""

git push

for target in $targets
do
  echo building for $target
  cargo build --release --target $target
  files="$files -a ./target/$target/release/dmux#dmux-$version-$target.tar.gz"
done
echo $files

hub release create $files -m $version --edit $version
