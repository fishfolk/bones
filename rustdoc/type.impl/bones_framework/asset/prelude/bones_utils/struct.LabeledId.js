(function() {var type_impls = {
"bones_framework":[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-LabeledId\" class=\"impl\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#47\">source</a><a href=\"#impl-LabeledId\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html\" title=\"struct bones_framework::asset::prelude::bones_utils::LabeledId\">LabeledId</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.new\" class=\"method\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#49\">source</a><h4 class=\"code-header\">pub fn <a href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html#tymethod.new\" class=\"fn\">new</a>(prefix: &amp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.str.html\">str</a>) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.75.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html\" title=\"struct bones_framework::asset::prelude::bones_utils::LabeledId\">LabeledId</a>, <a class=\"enum\" href=\"bones_framework/asset/prelude/bones_utils/enum.LabeledIdCreateError.html\" title=\"enum bones_framework::asset::prelude::bones_utils::LabeledIdCreateError\">LabeledIdCreateError</a>&gt;</h4></section></summary><div class=\"docblock\"><p>Create a new labeled ID with the given prefix.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.new_with_ulid\" class=\"method\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#54\">source</a><h4 class=\"code-header\">pub fn <a href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html#tymethod.new_with_ulid\" class=\"fn\">new_with_ulid</a>(\n    prefix: &amp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.str.html\">str</a>,\n    ulid: <a class=\"struct\" href=\"bones_framework/asset/struct.Ulid.html\" title=\"struct bones_framework::asset::Ulid\">Ulid</a>\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.75.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html\" title=\"struct bones_framework::asset::prelude::bones_utils::LabeledId\">LabeledId</a>, <a class=\"enum\" href=\"bones_framework/asset/prelude/bones_utils/enum.LabeledIdCreateError.html\" title=\"enum bones_framework::asset::prelude::bones_utils::LabeledIdCreateError\">LabeledIdCreateError</a>&gt;</h4></section></summary><div class=\"docblock\"><p>Create a new labeled ID with the given prefix and ULID.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.prefix\" class=\"method\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#73\">source</a><h4 class=\"code-header\">pub fn <a href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html#tymethod.prefix\" class=\"fn\">prefix</a>(&amp;self) -&gt; &amp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.str.html\">str</a></h4></section></summary><div class=\"docblock\"><p>Get the prefix of the ID.</p>\n</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.ulid\" class=\"method\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#85\">source</a><h4 class=\"code-header\">pub fn <a href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html#tymethod.ulid\" class=\"fn\">ulid</a>(&amp;self) -&gt; <a class=\"struct\" href=\"bones_framework/asset/struct.Ulid.html\" title=\"struct bones_framework::asset::Ulid\">Ulid</a></h4></section></summary><div class=\"docblock\"><p>Get the <a href=\"bones_framework/asset/struct.Ulid.html\" title=\"struct bones_framework::asset::Ulid\"><code>Ulid</code></a> of the ID.</p>\n</div></details></div></details>",0,"bones_framework::asset::AssetPackId"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Deserialize%3C'de%3E-for-LabeledId\" class=\"impl\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#169\">source</a><a href=\"#impl-Deserialize%3C'de%3E-for-LabeledId\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;'de&gt; <a class=\"trait\" href=\"bones_framework/prelude/trait.Deserialize.html\" title=\"trait bones_framework::prelude::Deserialize\">Deserialize</a>&lt;'de&gt; for <a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html\" title=\"struct bones_framework::asset::prelude::bones_utils::LabeledId\">LabeledId</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.deserialize\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#170-172\">source</a><a href=\"#method.deserialize\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"bones_framework/prelude/trait.Deserialize.html#tymethod.deserialize\" class=\"fn\">deserialize</a>&lt;D&gt;(\n    deserializer: D\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.75.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html\" title=\"struct bones_framework::asset::prelude::bones_utils::LabeledId\">LabeledId</a>, &lt;D as <a class=\"trait\" href=\"https://docs.rs/serde/1.0.195/serde/de/trait.Deserializer.html\" title=\"trait serde::de::Deserializer\">Deserializer</a>&lt;'de&gt;&gt;::<a class=\"associatedtype\" href=\"https://docs.rs/serde/1.0.195/serde/de/trait.Deserializer.html#associatedtype.Error\" title=\"type serde::de::Deserializer::Error\">Error</a>&gt;<span class=\"where fmt-newline\">where\n    D: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.195/serde/de/trait.Deserializer.html\" title=\"trait serde::de::Deserializer\">Deserializer</a>&lt;'de&gt;,</span></h4></section></summary><div class='docblock'>Deserialize this value from the given Serde deserializer. <a href=\"bones_framework/prelude/trait.Deserialize.html#tymethod.deserialize\">Read more</a></div></details></div></details>","Deserialize<'de>","bones_framework::asset::AssetPackId"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Debug-for-LabeledId\" class=\"impl\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#19\">source</a><a href=\"#impl-Debug-for-LabeledId\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"bones_framework/asset/prelude/bones_utils/prelude/alloc/fmt/trait.Debug.html\" title=\"trait bones_framework::asset::prelude::bones_utils::prelude::alloc::fmt::Debug\">Debug</a> for <a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html\" title=\"struct bones_framework::asset::prelude::bones_utils::LabeledId\">LabeledId</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#20\">source</a><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"bones_framework/asset/prelude/bones_utils/prelude/alloc/fmt/trait.Debug.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/prelude/alloc/fmt/struct.Formatter.html\" title=\"struct bones_framework::asset::prelude::bones_utils::prelude::alloc::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.75.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/prelude/alloc/fmt/struct.Error.html\" title=\"struct bones_framework::asset::prelude::bones_utils::prelude::alloc::fmt::Error\">Error</a>&gt;</h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"bones_framework/asset/prelude/bones_utils/prelude/alloc/fmt/trait.Debug.html#tymethod.fmt\">Read more</a></div></details></div></details>","Debug","bones_framework::asset::AssetPackId"],["<section id=\"impl-StructuralEq-for-LabeledId\" class=\"impl\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#11\">source</a><a href=\"#impl-StructuralEq-for-LabeledId\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/marker/trait.StructuralEq.html\" title=\"trait core::marker::StructuralEq\">StructuralEq</a> for <a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html\" title=\"struct bones_framework::asset::prelude::bones_utils::LabeledId\">LabeledId</a></h3></section>","StructuralEq","bones_framework::asset::AssetPackId"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Clone-for-LabeledId\" class=\"impl\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#11\">source</a><a href=\"#impl-Clone-for-LabeledId\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> for <a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html\" title=\"struct bones_framework::asset::prelude::bones_utils::LabeledId\">LabeledId</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#11\">source</a><a href=\"#method.clone\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.75.0/core/clone/trait.Clone.html#tymethod.clone\" class=\"fn\">clone</a>(&amp;self) -&gt; <a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html\" title=\"struct bones_framework::asset::prelude::bones_utils::LabeledId\">LabeledId</a></h4></section></summary><div class='docblock'>Returns a copy of the value. <a href=\"https://doc.rust-lang.org/1.75.0/core/clone/trait.Clone.html#tymethod.clone\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone_from\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.75.0/src/core/clone.rs.html#169\">source</a></span><a href=\"#method.clone_from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.75.0/core/clone/trait.Clone.html#method.clone_from\" class=\"fn\">clone_from</a>(&amp;mut self, source: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.reference.html\">&amp;Self</a>)</h4></section></summary><div class='docblock'>Performs copy-assignment from <code>source</code>. <a href=\"https://doc.rust-lang.org/1.75.0/core/clone/trait.Clone.html#method.clone_from\">Read more</a></div></details></div></details>","Clone","bones_framework::asset::AssetPackId"],["<section id=\"impl-Eq-for-LabeledId\" class=\"impl\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#11\">source</a><a href=\"#impl-Eq-for-LabeledId\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/cmp/trait.Eq.html\" title=\"trait core::cmp::Eq\">Eq</a> for <a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html\" title=\"struct bones_framework::asset::prelude::bones_utils::LabeledId\">LabeledId</a></h3></section>","Eq","bones_framework::asset::AssetPackId"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-PartialOrd-for-LabeledId\" class=\"impl\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#11\">source</a><a href=\"#impl-PartialOrd-for-LabeledId\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/cmp/trait.PartialOrd.html\" title=\"trait core::cmp::PartialOrd\">PartialOrd</a> for <a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html\" title=\"struct bones_framework::asset::prelude::bones_utils::LabeledId\">LabeledId</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.partial_cmp\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#11\">source</a><a href=\"#method.partial_cmp\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.75.0/core/cmp/trait.PartialOrd.html#tymethod.partial_cmp\" class=\"fn\">partial_cmp</a>(&amp;self, other: &amp;<a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html\" title=\"struct bones_framework::asset::prelude::bones_utils::LabeledId\">LabeledId</a>) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.75.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/1.75.0/core/cmp/enum.Ordering.html\" title=\"enum core::cmp::Ordering\">Ordering</a>&gt;</h4></section></summary><div class='docblock'>This method returns an ordering between <code>self</code> and <code>other</code> values if one exists. <a href=\"https://doc.rust-lang.org/1.75.0/core/cmp/trait.PartialOrd.html#tymethod.partial_cmp\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.lt\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.75.0/src/core/cmp.rs.html#1122\">source</a></span><a href=\"#method.lt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.75.0/core/cmp/trait.PartialOrd.html#method.lt\" class=\"fn\">lt</a>(&amp;self, other: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.reference.html\">&amp;Rhs</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>This method tests less than (for <code>self</code> and <code>other</code>) and is used by the <code>&lt;</code> operator. <a href=\"https://doc.rust-lang.org/1.75.0/core/cmp/trait.PartialOrd.html#method.lt\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.le\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.75.0/src/core/cmp.rs.html#1139\">source</a></span><a href=\"#method.le\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.75.0/core/cmp/trait.PartialOrd.html#method.le\" class=\"fn\">le</a>(&amp;self, other: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.reference.html\">&amp;Rhs</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>This method tests less than or equal to (for <code>self</code> and <code>other</code>) and is used by the <code>&lt;=</code>\noperator. <a href=\"https://doc.rust-lang.org/1.75.0/core/cmp/trait.PartialOrd.html#method.le\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.gt\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.75.0/src/core/cmp.rs.html#1155\">source</a></span><a href=\"#method.gt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.75.0/core/cmp/trait.PartialOrd.html#method.gt\" class=\"fn\">gt</a>(&amp;self, other: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.reference.html\">&amp;Rhs</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>This method tests greater than (for <code>self</code> and <code>other</code>) and is used by the <code>&gt;</code> operator. <a href=\"https://doc.rust-lang.org/1.75.0/core/cmp/trait.PartialOrd.html#method.gt\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.ge\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.75.0/src/core/cmp.rs.html#1172\">source</a></span><a href=\"#method.ge\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.75.0/core/cmp/trait.PartialOrd.html#method.ge\" class=\"fn\">ge</a>(&amp;self, other: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.reference.html\">&amp;Rhs</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>This method tests greater than or equal to (for <code>self</code> and <code>other</code>) and is used by the <code>&gt;=</code>\noperator. <a href=\"https://doc.rust-lang.org/1.75.0/core/cmp/trait.PartialOrd.html#method.ge\">Read more</a></div></details></div></details>","PartialOrd","bones_framework::asset::AssetPackId"],["<section id=\"impl-StructuralPartialEq-for-LabeledId\" class=\"impl\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#11\">source</a><a href=\"#impl-StructuralPartialEq-for-LabeledId\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/marker/trait.StructuralPartialEq.html\" title=\"trait core::marker::StructuralPartialEq\">StructuralPartialEq</a> for <a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html\" title=\"struct bones_framework::asset::prelude::bones_utils::LabeledId\">LabeledId</a></h3></section>","StructuralPartialEq","bones_framework::asset::AssetPackId"],["<section id=\"impl-Copy-for-LabeledId\" class=\"impl\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#11\">source</a><a href=\"#impl-Copy-for-LabeledId\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/marker/trait.Copy.html\" title=\"trait core::marker::Copy\">Copy</a> for <a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html\" title=\"struct bones_framework::asset::prelude::bones_utils::LabeledId\">LabeledId</a></h3></section>","Copy","bones_framework::asset::AssetPackId"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Ord-for-LabeledId\" class=\"impl\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#11\">source</a><a href=\"#impl-Ord-for-LabeledId\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/cmp/trait.Ord.html\" title=\"trait core::cmp::Ord\">Ord</a> for <a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html\" title=\"struct bones_framework::asset::prelude::bones_utils::LabeledId\">LabeledId</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.cmp\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#11\">source</a><a href=\"#method.cmp\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.75.0/core/cmp/trait.Ord.html#tymethod.cmp\" class=\"fn\">cmp</a>(&amp;self, other: &amp;<a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html\" title=\"struct bones_framework::asset::prelude::bones_utils::LabeledId\">LabeledId</a>) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.75.0/core/cmp/enum.Ordering.html\" title=\"enum core::cmp::Ordering\">Ordering</a></h4></section></summary><div class='docblock'>This method returns an <a href=\"https://doc.rust-lang.org/1.75.0/core/cmp/enum.Ordering.html\" title=\"enum core::cmp::Ordering\"><code>Ordering</code></a> between <code>self</code> and <code>other</code>. <a href=\"https://doc.rust-lang.org/1.75.0/core/cmp/trait.Ord.html#tymethod.cmp\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.max\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.21.0\">1.21.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.75.0/src/core/cmp.rs.html#826-828\">source</a></span><a href=\"#method.max\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.75.0/core/cmp/trait.Ord.html#method.max\" class=\"fn\">max</a>(self, other: Self) -&gt; Self<span class=\"where fmt-newline\">where\n    Self: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</span></h4></section></summary><div class='docblock'>Compares and returns the maximum of two values. <a href=\"https://doc.rust-lang.org/1.75.0/core/cmp/trait.Ord.html#method.max\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.min\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.21.0\">1.21.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.75.0/src/core/cmp.rs.html#846-848\">source</a></span><a href=\"#method.min\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.75.0/core/cmp/trait.Ord.html#method.min\" class=\"fn\">min</a>(self, other: Self) -&gt; Self<span class=\"where fmt-newline\">where\n    Self: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</span></h4></section></summary><div class='docblock'>Compares and returns the minimum of two values. <a href=\"https://doc.rust-lang.org/1.75.0/core/cmp/trait.Ord.html#method.min\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.clamp\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.50.0\">1.50.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.75.0/src/core/cmp.rs.html#871-874\">source</a></span><a href=\"#method.clamp\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.75.0/core/cmp/trait.Ord.html#method.clamp\" class=\"fn\">clamp</a>(self, min: Self, max: Self) -&gt; Self<span class=\"where fmt-newline\">where\n    Self: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/cmp/trait.PartialOrd.html\" title=\"trait core::cmp::PartialOrd\">PartialOrd</a>,</span></h4></section></summary><div class='docblock'>Restrict a value to a certain interval. <a href=\"https://doc.rust-lang.org/1.75.0/core/cmp/trait.Ord.html#method.clamp\">Read more</a></div></details></div></details>","Ord","bones_framework::asset::AssetPackId"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Serialize-for-LabeledId\" class=\"impl\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#160\">source</a><a href=\"#impl-Serialize-for-LabeledId\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"bones_framework/prelude/trait.Serialize.html\" title=\"trait bones_framework::prelude::Serialize\">Serialize</a> for <a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html\" title=\"struct bones_framework::asset::prelude::bones_utils::LabeledId\">LabeledId</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.serialize\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#161-163\">source</a><a href=\"#method.serialize\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"bones_framework/prelude/trait.Serialize.html#tymethod.serialize\" class=\"fn\">serialize</a>&lt;S&gt;(\n    &amp;self,\n    serializer: S\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.75.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;&lt;S as <a class=\"trait\" href=\"https://docs.rs/serde/1.0.195/serde/ser/trait.Serializer.html\" title=\"trait serde::ser::Serializer\">Serializer</a>&gt;::<a class=\"associatedtype\" href=\"https://docs.rs/serde/1.0.195/serde/ser/trait.Serializer.html#associatedtype.Ok\" title=\"type serde::ser::Serializer::Ok\">Ok</a>, &lt;S as <a class=\"trait\" href=\"https://docs.rs/serde/1.0.195/serde/ser/trait.Serializer.html\" title=\"trait serde::ser::Serializer\">Serializer</a>&gt;::<a class=\"associatedtype\" href=\"https://docs.rs/serde/1.0.195/serde/ser/trait.Serializer.html#associatedtype.Error\" title=\"type serde::ser::Serializer::Error\">Error</a>&gt;<span class=\"where fmt-newline\">where\n    S: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.195/serde/ser/trait.Serializer.html\" title=\"trait serde::ser::Serializer\">Serializer</a>,</span></h4></section></summary><div class='docblock'>Serialize this value into the given Serde serializer. <a href=\"bones_framework/prelude/trait.Serialize.html#tymethod.serialize\">Read more</a></div></details></div></details>","Serialize","bones_framework::asset::AssetPackId"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Display-for-LabeledId\" class=\"impl\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#98\">source</a><a href=\"#impl-Display-for-LabeledId\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"bones_framework/asset/prelude/bones_utils/prelude/alloc/fmt/trait.Display.html\" title=\"trait bones_framework::asset::prelude::bones_utils::prelude::alloc::fmt::Display\">Display</a> for <a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html\" title=\"struct bones_framework::asset::prelude::bones_utils::LabeledId\">LabeledId</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.fmt\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#99\">source</a><a href=\"#method.fmt\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"bones_framework/asset/prelude/bones_utils/prelude/alloc/fmt/trait.Display.html#tymethod.fmt\" class=\"fn\">fmt</a>(&amp;self, f: &amp;mut <a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/prelude/alloc/fmt/struct.Formatter.html\" title=\"struct bones_framework::asset::prelude::bones_utils::prelude::alloc::fmt::Formatter\">Formatter</a>&lt;'_&gt;) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.75.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.unit.html\">()</a>, <a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/prelude/alloc/fmt/struct.Error.html\" title=\"struct bones_framework::asset::prelude::bones_utils::prelude::alloc::fmt::Error\">Error</a>&gt;</h4></section></summary><div class='docblock'>Formats the value using the given formatter. <a href=\"bones_framework/asset/prelude/bones_utils/prelude/alloc/fmt/trait.Display.html#tymethod.fmt\">Read more</a></div></details></div></details>","Display","bones_framework::asset::AssetPackId"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-FromStr-for-LabeledId\" class=\"impl\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#140\">source</a><a href=\"#impl-FromStr-for-LabeledId\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"bones_framework/asset/prelude/bones_utils/prelude/alloc/str/trait.FromStr.html\" title=\"trait bones_framework::asset::prelude::bones_utils::prelude::alloc::str::FromStr\">FromStr</a> for <a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html\" title=\"struct bones_framework::asset::prelude::bones_utils::LabeledId\">LabeledId</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle\" open><summary><section id=\"associatedtype.Err\" class=\"associatedtype trait-impl\"><a href=\"#associatedtype.Err\" class=\"anchor\">§</a><h4 class=\"code-header\">type <a href=\"bones_framework/asset/prelude/bones_utils/prelude/alloc/str/trait.FromStr.html#associatedtype.Err\" class=\"associatedtype\">Err</a> = <a class=\"enum\" href=\"bones_framework/asset/prelude/bones_utils/enum.LabledIdParseError.html\" title=\"enum bones_framework::asset::prelude::bones_utils::LabledIdParseError\">LabledIdParseError</a></h4></section></summary><div class='docblock'>The associated error which can be returned from parsing.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.from_str\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#143\">source</a><a href=\"#method.from_str\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"bones_framework/asset/prelude/bones_utils/prelude/alloc/str/trait.FromStr.html#tymethod.from_str\" class=\"fn\">from_str</a>(s: &amp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.str.html\">str</a>) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.75.0/core/result/enum.Result.html\" title=\"enum core::result::Result\">Result</a>&lt;<a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html\" title=\"struct bones_framework::asset::prelude::bones_utils::LabeledId\">LabeledId</a>, &lt;<a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html\" title=\"struct bones_framework::asset::prelude::bones_utils::LabeledId\">LabeledId</a> as <a class=\"trait\" href=\"bones_framework/asset/prelude/bones_utils/prelude/alloc/str/trait.FromStr.html\" title=\"trait bones_framework::asset::prelude::bones_utils::prelude::alloc::str::FromStr\">FromStr</a>&gt;::<a class=\"associatedtype\" href=\"bones_framework/asset/prelude/bones_utils/prelude/alloc/str/trait.FromStr.html#associatedtype.Err\" title=\"type bones_framework::asset::prelude::bones_utils::prelude::alloc::str::FromStr::Err\">Err</a>&gt;</h4></section></summary><div class='docblock'>Parses a string <code>s</code> to return a value of this type. <a href=\"bones_framework/asset/prelude/bones_utils/prelude/alloc/str/trait.FromStr.html#tymethod.from_str\">Read more</a></div></details></div></details>","FromStr","bones_framework::asset::AssetPackId"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-PartialEq-for-LabeledId\" class=\"impl\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#11\">source</a><a href=\"#impl-PartialEq-for-LabeledId\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a> for <a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html\" title=\"struct bones_framework::asset::prelude::bones_utils::LabeledId\">LabeledId</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.eq\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#11\">source</a><a href=\"#method.eq\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.75.0/core/cmp/trait.PartialEq.html#tymethod.eq\" class=\"fn\">eq</a>(&amp;self, other: &amp;<a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html\" title=\"struct bones_framework::asset::prelude::bones_utils::LabeledId\">LabeledId</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>This method tests for <code>self</code> and <code>other</code> values to be equal, and is used\nby <code>==</code>.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.ne\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.75.0/src/core/cmp.rs.html#239\">source</a></span><a href=\"#method.ne\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.75.0/core/cmp/trait.PartialEq.html#method.ne\" class=\"fn\">ne</a>(&amp;self, other: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.reference.html\">&amp;Rhs</a>) -&gt; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.bool.html\">bool</a></h4></section></summary><div class='docblock'>This method tests for <code>!=</code>. The default implementation is almost always\nsufficient, and should not be overridden without very good reason.</div></details></div></details>","PartialEq","bones_framework::asset::AssetPackId"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Hash-for-LabeledId\" class=\"impl\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#11\">source</a><a href=\"#impl-Hash-for-LabeledId\" class=\"anchor\">§</a><h3 class=\"code-header\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/hash/trait.Hash.html\" title=\"trait core::hash::Hash\">Hash</a> for <a class=\"struct\" href=\"bones_framework/asset/prelude/bones_utils/struct.LabeledId.html\" title=\"struct bones_framework::asset::prelude::bones_utils::LabeledId\">LabeledId</a></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.hash\" class=\"method trait-impl\"><a class=\"src rightside\" href=\"src/bones_utils/labeled_id.rs.html#11\">source</a><a href=\"#method.hash\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.75.0/core/hash/trait.Hash.html#tymethod.hash\" class=\"fn\">hash</a>&lt;__H&gt;(&amp;self, state: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.reference.html\">&amp;mut __H</a>)<span class=\"where fmt-newline\">where\n    __H: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\">Hasher</a>,</span></h4></section></summary><div class='docblock'>Feeds this value into the given <a href=\"https://doc.rust-lang.org/1.75.0/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\"><code>Hasher</code></a>. <a href=\"https://doc.rust-lang.org/1.75.0/core/hash/trait.Hash.html#tymethod.hash\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.hash_slice\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.3.0\">1.3.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.75.0/src/core/hash/mod.rs.html#242-244\">source</a></span><a href=\"#method.hash_slice\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.75.0/core/hash/trait.Hash.html#method.hash_slice\" class=\"fn\">hash_slice</a>&lt;H&gt;(data: &amp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.slice.html\">[Self]</a>, state: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.75.0/std/primitive.reference.html\">&amp;mut H</a>)<span class=\"where fmt-newline\">where\n    H: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\">Hasher</a>,\n    Self: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.75.0/core/marker/trait.Sized.html\" title=\"trait core::marker::Sized\">Sized</a>,</span></h4></section></summary><div class='docblock'>Feeds a slice of this type into the given <a href=\"https://doc.rust-lang.org/1.75.0/core/hash/trait.Hasher.html\" title=\"trait core::hash::Hasher\"><code>Hasher</code></a>. <a href=\"https://doc.rust-lang.org/1.75.0/core/hash/trait.Hash.html#method.hash_slice\">Read more</a></div></details></div></details>","Hash","bones_framework::asset::AssetPackId"]]
};if (window.register_type_impls) {window.register_type_impls(type_impls);} else {window.pending_type_impls = type_impls;}})()