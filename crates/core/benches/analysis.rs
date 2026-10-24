use std::fs;
use std::path::{Path, PathBuf};

use codeatlas_core::pipeline::{analyze, AnalyzeOptions};
use criterion::{criterion_group, criterion_main, Criterion};

struct TempRepo {
    root: PathBuf,
}

impl TempRepo {
    fn new(file_count: usize) -> Self {
        let root = std::env::temp_dir().join(format!(
            "codeatlas-bench-{}-{}",
            std::process::id(),
            file_count
        ));
        let _ = fs::remove_dir_all(&root);
        for index in 0..file_count {
            let package = root.join(format!("pkg{}", index % 10));
            fs::create_dir_all(&package).expect("create benchmark package");
            let source = format!(
                "import os\n\nclass Widget{index}:\n    def run(self, value):\n        return helper(value)\n\ndef helper(value):\n    return value + {index}\n"
            );
            fs::write(package.join(format!("module_{index}.py")), source)
                .expect("write benchmark source file");
        }
        Self { root }
    }

    fn path(&self) -> &Path {
        &self.root
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn bench_pipeline(c: &mut Criterion) {
    let repo = TempRepo::new(200);
    let options = AnalyzeOptions {
        repo_path: repo.path().to_path_buf(),
        project_name: "bench".to_string(),
        depth_cap: 3,
        ignore_paths: Vec::new(),
    };

    c.bench_function("analyze_200_python_files", |b| {
        b.iter(|| {
            let result = analyze(&options).expect("benchmark analysis succeeds");
            criterion::black_box(result.analysis.files.len());
        })
    });
}

criterion_group!(benches, bench_pipeline);
criterion_main!(benches);
