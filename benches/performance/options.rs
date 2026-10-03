use super::*;
pub(super) struct Options {
    pub(super) iterations: usize,
    pub(super) warmup: usize,
    pub(super) sizes: Vec<usize>,
    pub(super) size: PhysicalSize<u32>,
    pub(super) cpu: bool,
    pub(super) gpu: bool,
    pub(super) gpu_wait: bool,
    pub(super) gpu_timestamps: bool,
    pub(super) filter: String,
    pub(super) output: PathBuf,
    pub(super) max_p95: Option<f64>,
}

impl Options {
    pub(super) fn parse() -> BenchResult<Option<Self>> {
        let mut options = Self {
            iterations: 120,
            warmup: 30,
            sizes: vec![32, 128, 512],
            size: PhysicalSize::new(1280, 800),
            cpu: true,
            gpu: true,
            gpu_wait: false,
            gpu_timestamps: false,
            filter: String::new(),
            output: PathBuf::from("target/benchmark.json"),
            max_p95: None,
        };
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--bench" => {} // Cargo may append the libtest flag to custom harnesses.
                "--help" | "-h" => {
                    println!("zaxis performance benchmark (release, real desktop GPU by default)\n\
                        --quick                  12 samples, 3 warmup frames, 32 objects\n\
                        --hard                   600 samples, 120 warmup frames, 128/512/2048 objects\n\
                        --cpu-only / --gpu-only   select suite (CPU needs no display)\n\
                        --iterations N           measured frames per case/size/mode (default 120)\n\
                        --warmup N               untimed warmup frames (default 30, minimum 1)\n\
                        --sizes N,N,...          objects per stress scene (default 32,128,512)\n\
                        --resolution WxH         physical window size (default 1280x800)\n\
                        --filter TEXT            comma-separated case substrings; --list for names\n\
                        --gpu-wait               serialize each frame until GPU work completes\n\
                        --gpu-timestamps         diagnostic pass/copy timestamps and serialized readback\n\
                        --output PATH            JSON with all distributions and counters\n\
                        --max-p95-ms N           fail if total frame p95 exceeds this budget\n\
                        --list                   list cases and exit\n\n\
                        Vsync and Immediate both run. Immediate can fall back; capabilities are reported.\n\
                        Render includes acquisition, encoding, submission and present, not GPU-only time.\n\
                        Synthetic input measures dispatcher-to-model/frame latency, not OS or display latency.");
                    return Ok(None);
                }
                "--list" => {
                    for case in Case::ALL {
                        println!("{}", case.name());
                    }
                    return Ok(None);
                }
                "--quick" => {
                    options.iterations = 12;
                    options.warmup = 3;
                    options.sizes = vec![32];
                }
                "--hard" => {
                    options.iterations = 600;
                    options.warmup = 120;
                    options.sizes = vec![128, 512, 2048];
                }
                "--cpu-only" => {
                    options.cpu = true;
                    options.gpu = false;
                }
                "--gpu-only" => {
                    options.cpu = false;
                    options.gpu = true;
                }
                "--gpu-wait" => options.gpu_wait = true,
                "--gpu-timestamps" => options.gpu_timestamps = true,
                "--iterations" => {
                    options.iterations = args.next().ok_or("missing iterations")?.parse()?
                }
                "--warmup" => options.warmup = args.next().ok_or("missing warmup")?.parse()?,
                "--sizes" => {
                    options.sizes = args
                        .next()
                        .ok_or("missing sizes")?
                        .split(',')
                        .map(str::parse)
                        .collect::<Result<_, _>>()?
                }
                "--filter" => options.filter = args.next().ok_or("missing filter")?,
                "--output" => options.output = args.next().ok_or("missing output")?.into(),
                "--max-p95-ms" => {
                    options.max_p95 = Some(args.next().ok_or("missing p95 budget")?.parse()?)
                }
                "--resolution" => {
                    let value = args.next().ok_or("missing resolution")?;
                    let (w, h) = value.split_once('x').ok_or("expected WIDTHxHEIGHT")?;
                    options.size = PhysicalSize::new(w.parse()?, h.parse()?);
                }
                _ => return Err(format!("unknown option {arg}; use --help").into()),
            }
        }
        if cfg!(debug_assertions) {
            return Err("run benchmarks in release mode with cargo bench".into());
        }
        if options.iterations < 2
            || options.warmup == 0
            || options.sizes.is_empty()
            || options.sizes.contains(&0)
        {
            return Err("iterations must be >= 2, warmup and every size must be >= 1".into());
        }
        if options.size.width < 640 || options.size.height < 480 {
            return Err("resolution must be at least 640x480 for the interaction probe".into());
        }
        if options.max_p95.is_some_and(|p| !p.is_finite() || p <= 0.0) {
            return Err("p95 budget must be finite and positive".into());
        }
        Ok(Some(options))
    }
    pub(super) fn cases(&self) -> Vec<Case> {
        Case::ALL
            .into_iter()
            .filter(|c| {
                self.filter
                    .split(',')
                    .any(|filter| c.name().contains(filter))
            })
            .collect()
    }
}
