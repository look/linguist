require_relative "./helper"

# This test verifies that the Rust bindings work.
class TestRust < Minitest::Test
  def setup
    require "linguist/linguist"
  end

  def test_detect
    id = Linguist::Rust.detect('src/main.rs', 'fn main() { println!("Hello, world!");}')
    assert_equal "Rust", Linguist::Language.find_by_id(id).name
  end

  def test_is_test_positive
    assert Linguist::Rust.is_test?("test/api_test.rb")
    assert Linguist::Rust.is_test?("tests/unit/foo.py")
    assert Linguist::Rust.is_test?("src/tests/helper.rs")
    assert Linguist::Rust.is_test?("snapshots/foo.snap")
  end

  def test_is_test_negative
    refute Linguist::Rust.is_test?("testfile.rb")
    refute Linguist::Rust.is_test?("src/main.rs")
    refute Linguist::Rust.is_test?("lib/foo.rb")
  end

  def test_is_documentation_positive
    assert Linguist::Rust.is_documentation?("README")
    assert Linguist::Rust.is_documentation?("README.md")
    assert Linguist::Rust.is_documentation?("CHANGELOG.md")
  end

  def test_is_documentation_negative
    refute Linguist::Rust.is_documentation?("foo.rb")
    refute Linguist::Rust.is_documentation?("src/main.rs")
  end

  def test_is_dependency_management_positive
    assert Linguist::Rust.is_dependency_management?("Gemfile")
    assert Linguist::Rust.is_dependency_management?("package.json")
    assert Linguist::Rust.is_dependency_management?("Cargo.toml")
    assert Linguist::Rust.is_dependency_management?("requirements.txt")
    assert Linguist::Rust.is_dependency_management?("go.mod")
  end

  def test_is_dependency_management_negative
    refute Linguist::Rust.is_dependency_management?("main.rs")
    refute Linguist::Rust.is_dependency_management?("lib.rs")
    refute Linguist::Rust.is_dependency_management?("README.md")
  end
end
