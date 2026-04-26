use anyhow::Result;
use qmetaobject::*;

#[derive(QObject, Default)]
pub struct QtDemoBridge {
    base: qt_base_class!(trait QObject),
    test_value: qt_property!(i32),
    get_player_info: qt_method!(fn(&self) -> QVariantMap),
}

impl QtDemoBridge {
    pub fn new() -> Self {
        Self {
            base: Default::default(),
            test_value: 42,
            get_player_info: Default::default(),
        }
    }

    pub fn initialize(&mut self) -> Result<()> {
        tracing::info!("Initializing QtDemoBridge");
        Ok(())
    }

    // Qt method to provide basic info for QML testing
    fn get_player_info(&self) -> QVariantMap {
        let mut map = QVariantMap::default();
        map.insert(
            "version".into(),
            QVariant::from(QString::from("qt-demo-1.0.0")),
        );
        map.insert("backend".into(), QVariant::from(QString::from("qt-demo")));
        map.insert("test_value".into(), QVariant::from(self.test_value));
        map.insert("status".into(), QVariant::from(QString::from("demo-ready")));
        map
    }
}
