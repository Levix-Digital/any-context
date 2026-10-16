//! Real cross-encoder validation (ADR-130).
//!
//! Downloads the verified `mmarco-reranker` (~136 MB, cached) and checks multilingual relevance
//! and CPU latency with the real ONNX Runtime. Opt-in, never runs in the default suite:
//!
//! ```text
//! ORT_DYLIB_PATH=<path to onnxruntime library> ACTX_TEST_REAL_RERANKER=1 \
//!   cargo test -p any-context-core-rs --test reranker_real_tests -- --ignored --nocapture
//! ```

use any_context_core_rs::ingestion::{OnnxModelManager, DEFAULT_RERANKER_ID};

fn percentile(sorted: &[u128], p: f64) -> u128 {
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[idx]
}

#[test]
#[ignore]
fn real_reranker_multilingual() {
    if std::env::var("ACTX_TEST_REAL_RERANKER").as_deref() != Ok("1") {
        eprintln!("skipping: set ACTX_TEST_REAL_RERANKER=1");
        return;
    }
    if std::env::var("ACTX_MODELS_DIR").is_err() {
        std::env::set_var("ACTX_MODELS_DIR", std::env::temp_dir().join("actx_real_models"));
    }
    actx_onnx::runtime::probe_runtime().expect("ONNX Runtime library must be loadable (ORT_DYLIB_PATH)");

    let spec = OnnxModelManager::find_spec(DEFAULT_RERANKER_ID).unwrap();
    if !OnnxModelManager::is_installed(spec) {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(OnnxModelManager::download_model(spec)).expect("verified download");
    }
    assert!(OnnxModelManager::is_installed(spec));

    let reranker = actx_onnx::OnnxReranker::new(&OnnxModelManager::model_path(spec));
    let load_started = std::time::Instant::now();
    reranker.ensure_loaded().expect("real model loads");
    println!("model load: {} ms", load_started.elapsed().as_millis());

    // Portuguese query; distractors share vocabulary (version, negation, other module).
    let query = "Qual é o timeout padrão das requisições síncronas no serviço de pagamento v2?";
    let docs = vec![
        "No serviço de pagamento v1 (legado), o timeout padrão de requisição síncrona era de 120 segundos.".to_string(),
        "O tema visual da interface usa as cores aurora boreal com gradientes azul e roxo.".to_string(),
        "No serviço de pagamento v2, o timeout padrão de requisições síncronas é de 30 segundos e retorna ERR_902_TIMEOUT.".to_string(),
        "Para webhooks do ecossistema v2, o timeout de conexão é de 5 segundos por tentativa.".to_string(),
    ];
    let scores = reranker.try_score_batch(query, &docs).expect("real inference");
    println!("scores: {:?}", scores);
    let best = scores
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, _)| i)
        .unwrap();
    assert_eq!(best, 2, "the v2 payment chunk must rank first: {:?}", scores);
    assert!(scores[2] > scores[1] + 0.2, "irrelevant theme chunk must score far lower");

    // Latency for a realistic Stage-2 pool of 30 candidates (~80-120 words each).
    let pool: Vec<String> = (0..30)
        .map(|i| format!("{} Seção {} da documentação técnica com detalhes adicionais de configuração, limites operacionais, políticas de retry e observabilidade do serviço. {}", docs[i % docs.len()], i, docs[(i + 1) % docs.len()]))
        .collect();
    let mut runs = Vec::new();
    for _ in 0..5 {
        let t = std::time::Instant::now();
        let s = reranker.try_score_batch(query, &pool).unwrap();
        assert_eq!(s.len(), 30);
        runs.push(t.elapsed().as_millis());
    }
    runs.sort_unstable();
    println!(
        "rerank 30 candidates on CPU ({} threads): p50 = {} ms, p95 = {} ms, runs = {:?}",
        actx_onnx::runtime::default_intra_threads(),
        percentile(&runs, 0.5),
        percentile(&runs, 0.95),
        runs
    );
}
