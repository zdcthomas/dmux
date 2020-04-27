args="$@"
version=${args[0]}
targets=("x86_64-apple-darwin")
files=""

git push

for target in $targets
do
  echo building for $target
  cargo build --release --target $target
  artifact=./target/$target/release/dmux
  shasum=$(shasum --algorithm 256 $artifact)
  files="$files -a $artifact#dmux-$version-$target.tar.gz"
  echo "sha256"
  echo "$shasum"
done
echo $files

hub release create $files -m $version --edit $version
