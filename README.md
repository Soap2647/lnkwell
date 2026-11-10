# Inkwell

Markadan bağımsız, açık kaynak masaüstü yazıcı uygulaması. Baskı, durum ve bakım tek pencerede; Windows, macOS ve Linux için (Tauri 2 + Rust).

> **Durum: ilk taslak.** "Inkwell" geçici çalışma adıdır.
>
> **Bu proje HP ile ya da başka bir yazıcı üreticisiyle bağlantılı değildir**, onlar tarafından desteklenmez veya onaylanmaz. Marka adları yalnızca uyumluluğu belirtmek için geçer.
>
> **Bakım işlemleri (kafa temizliği, kalite raporu) mürekkep ve kâğıt harcar.** Uygulama bunları yalnızca sen onaylayınca gönderir.

## Bu taslakta ne var

| Alan | Durum |
| --- | --- |
| Yazıcı durumu, uyarılar, baskı kafası parça kodları, sayaçlar | Çalışıyor (HP LEDM) |
| Cihazın desteklediği bakım işlerinin listesi | Çalışıyor (HP LEDM) |
| Bakım işi başlatma | Yazıldı, **gerçek cihazda henüz doğrulanmadı** |
| PDF önizleme (sürükle-bırak) ve baskı ayarları | Çalışıyor |
| Baskı gönderimi | Yok; `print` modülü yalnızca arayüz |
| Yazıcıyı ağda otomatik bulma | Yok; IP adresi elle girilir |

Test edilen cihaz: HP Smart Tank 510.

## Mimari

```
┌──────────────────────────── Arayüz (HTML/JS, Vite) ────────────────────────────┐
│  PDF önizleme · baskı ayarları · Yazıcı paneli (durum, uyarılar, bakım)        │
└───────────────────────────────────────┬────────────────────────────────────────┘
                                        │ Tauri komutları
┌───────────────────────────────────────┴────────────────────────────────────────┐
│ src-tauri            ince kabuk: printer_snapshot · run_maintenance · print_…  │
└───────────────────────────────────────┬────────────────────────────────────────┘
┌───────────────────────────────────────┴────────────────────────────────────────┐
│ crates/inkwell-core  (Tauri'ye bağımlı değil)                                  │
│                                                                                │
│   print                status                  maintenance                     │
│   IPP Everywhere,      trait StatusSource      trait PrinterDriver             │
│   yoksa işletim        durum, uyarı, sarf,     desteklenen işleri bildirir,    │
│   sistemi servisi      sayaç                   seçileni çalıştırır             │
│   (taslak: stub)             ▲                        ▲                        │
│                              └────────┬───────────────┘                        │
│   drivers/                            │                                        │
│     hp_ledm   ← şimdilik tek sürücü (XML / HTTP, /DevMgmt/…)                   │
│     …         ← sonraki markalar buraya                                        │
└────────────────────────────────────────────────────────────────────────────────┘
```

İlkeler:

- **Markaya özel kod yalnızca `drivers/` altında.** Çekirdek ve arayüz marka bilmez.
- **Sürücü yeteneklerini kendisi bildirir.** Arayüz yalnızca cihazın desteklediğini söylediği bakım işlerini gösterir; desteklenmeyen düğme hiç çizilmez.
- **Ölçülmeyen değer, ölçülmüş gibi gösterilmez.** Örneğin tanklı modeller seviyeyi sabit %100 bildirir; sürücü bunu `level_measured: false` olarak işaretler, arayüz yüzde yerine "gözle kontrol et" der.

### HP LEDM sürücüsü

Yazıcının gömülü web sunucusundaki XML uçlarını okur:

| Uç | İçerik |
| --- | --- |
| `GET /DevMgmt/ProductConfigDyn.xml` | Model, yazılım sürümü |
| `GET /DevMgmt/ProductStatusDyn.xml` | Durum ve uyarılar |
| `GET /DevMgmt/ConsumableConfigDyn.xml` | Baskı kafaları, tanklar/kartuşlar |
| `GET /DevMgmt/ProductUsageDyn.xml` | Sayaçlar |
| `GET /DevMgmt/InternalPrintCap.xml` | Desteklenen dahili işler |
| `POST /DevMgmt/InternalPrintDyn.xml` | Bakım işi başlatma (doğrulanmadı) |

Sürücü, yazıcının `InternalPrintCap.xml` içinde bildirmediği hiçbir iş türünü göndermez.

### Baskı yolu için notlar

Test cihazında gerçek bir IPP `Get-Printer-Attributes` isteğiyle doğrulandı:

- IPP 2.0, **631 portunda** `/ipp/print` (80 portunda yok); `ipp-everywhere` ve `airprint-1.8` bildiriyor.
- Kabul edilen biçimler: PWG Raster, URF, JPEG, PCLm. **PDF doğrudan kabul edilmiyor**; baskı için sayfaların uygulamada rasterleştirilmesi gerekecek.
- `sides-supported: one-sided`. Otomatik çift taraf yok; çift taraf elle çevirme akışıyla yapılmalı.

## Çalıştırma

Gerekenler: [Rust](https://rustup.rs), Node.js 20+, ve Tauri'nin [platform ön koşulları](https://tauri.app/start/prerequisites/) (Windows'ta MSVC C++ Build Tools ve WebView2).

```bash
npm install
npm run tauri dev
```

Çekirdek testleri (yazıcı gerekmez; kaydedilmiş yanıtlarla çalışır):

```bash
cargo test -p inkwell-core
```

Test verileri `crates/inkwell-core/tests/fixtures/` altındadır; seri numarası ve ağ adresleri gibi alanlar temizlenmiştir.

## Yol haritası (kısa)

- IPP Everywhere ile baskı gönderimi, işletim sistemi servisine geri düşme
- Ağda yazıcı bulma (mDNS)
- Elle çift taraf sihirbazı, kitapçık modu, hazır profiller
- Başka markalar için sürücüler

## Lisans

[MIT](LICENSE)
