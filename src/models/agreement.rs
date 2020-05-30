use vf_rs::vf;
basis_model! {
    pub struct Agreement {
        id: <<AgreementID>>,
        inner: vf::Agreement,
    }
    AgreementBuilder
}

