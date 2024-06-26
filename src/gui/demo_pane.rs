use eframe::egui::{self, FontId, TextBuffer, TextEdit, Ui, Widget};
use p256::elliptic_curve::generic_array::GenericArray;
use p256::elliptic_curve::point::AffineCoordinates;
use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::elliptic_curve::{Field, PrimeField};
use p256::ProjectivePoint;
use rand::thread_rng;
use sha2::{Digest, Sha256};

#[derive(Debug, Eq, PartialEq)]
enum C {
    C0,
    C1,
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct DemoPane {
    m0: String,
    m1: String,
    a: String,
    b: String,
    c: C,
    a_scalar: p256::Scalar,
    b_scalar: p256::Scalar,
    a_point: ProjectivePoint,
    b_point: ProjectivePoint,
    e0: Vec<u8>,
    e1: Vec<u8>,
}

impl DemoPane {
    pub(super) fn draw(&mut self, ui: &mut Ui) {
        ui.collapsing("Alice", |ui| {
            egui::Grid::new("alice").num_columns(2).show(ui, |ui| {
                ui.label("m0:");
                text_field(&mut self.m0).ui(ui);
                ui.end_row();
                ui.label("m1:");
                text_field(&mut self.m1).ui(ui);
                ui.end_row();
                ui.label("a:");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Random").clicked() {
                        let a = p256::Scalar::random(thread_rng());
                        self.a = format!("{:x}", a.to_bytes());
                    }
                    text_field(&mut self.a).ui(ui);
                });
                ui.end_row();
            });
        });
        ui.collapsing("Bob", |ui| {
            egui::Grid::new("bob").num_columns(2).show(ui, |ui| {
                ui.label("b:");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Random").clicked() {
                        let b = p256::Scalar::random(thread_rng());
                        self.b = format!("{:x}", b.to_bytes());
                    }
                    text_field(&mut self.b).ui(ui);
                });
                ui.end_row();
                ui.label("c:");
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.radio_value(&mut self.c, C::C0, "0");
                    ui.radio_value(&mut self.c, C::C1, "1");
                });
                ui.end_row();
            });
        });
        ui.collapsing("Oblivious Transfer Protocol (Alice -> Bob)", |ui| {
            let abytes = hex::decode(self.a.clone());
            if abytes.is_err() {
                ui.label("Invalid a");
                return;
            }
            let mut abytes = abytes.unwrap();
            if abytes.len() < 32 {
                let mut abytes2 = vec![0; 32 - abytes.len()];
                abytes2.append(&mut abytes);
                abytes = abytes2;
            }
            if abytes.len() > 32 {
                ui.label("a too long");
                return;
            }

            let abytes: [u8; 32] = abytes.try_into().unwrap();
            let abytes = GenericArray::from_slice(&abytes);
            self.a_scalar = p256::Scalar::from_repr(*abytes).unwrap();
            self.a_point = ProjectivePoint::GENERATOR * self.a_scalar;

            egui::Grid::new("a_to_b_1").num_columns(2).show(ui, |ui| {
                let a_point = self.a_point.to_affine();
                ui.label("A (x):");
                ui.label(format!("{:x}", a_point.x()));
                ui.end_row();
                ui.label("A (y) odd:");
                ui.label(format!("{}", a_point.y_is_odd().unwrap_u8()));
                ui.end_row();
            });
        });
        ui.collapsing("Oblivious Transfer Protocol (Bob -> Alice)", |ui| {
            let bbytes = hex::decode(self.b.clone());
            if bbytes.is_err() {
                ui.label("Invalid b");
                return;
            }
            let mut bbytes = bbytes.unwrap();
            if bbytes.len() < 32 {
                let mut bbytes2 = vec![0; 32 - bbytes.len()];
                bbytes2.append(&mut bbytes);
                bbytes = bbytes2;
            }
            if bbytes.len() > 32 {
                ui.label("b too long");
                return;
            }
            let bbytes = GenericArray::from_slice(&bbytes);
            self.b_scalar = p256::Scalar::from_repr(*bbytes).unwrap();

            let gen = ProjectivePoint::GENERATOR;

            self.b_point = if self.c == C::C0 {
                gen * self.b_scalar
            } else {
                self.a_point + gen * self.b_scalar
            };

            egui::Grid::new("b_to_a_1").num_columns(2).show(ui, |ui| {
                let b_point = self.b_point.to_affine();
                ui.label("B (x):");
                ui.label(format!("{:x}", b_point.x()));
                ui.end_row();
                ui.label("B (y) odd:");
                ui.label(format!("{}", b_point.y_is_odd().unwrap_u8()));
                ui.end_row();
            });
        });
        ui.collapsing("Oblivious Transfer Protocol (Alice -> Bob) ", |ui| {
            let k_0_p = self.b_point * self.a_scalar;
            let k_1_p = (self.b_point - self.a_point) * self.a_scalar;

            let k_0 = Sha256::digest(k_0_p.to_encoded_point(false).as_bytes())
                .as_slice()
                .try_into()
                .unwrap();
            let k_1 = Sha256::digest(k_1_p.to_encoded_point(false).as_bytes())
                .as_slice()
                .try_into()
                .unwrap();

            self.e0 = libaes::Cipher::new_256(&k_0).cbc_encrypt(&k_0, self.m0.as_bytes());
            self.e1 = libaes::Cipher::new_256(&k_1).cbc_encrypt(&k_1, self.m1.as_bytes());

            let e0 = hex::encode(&self.e0);
            let e1 = hex::encode(&self.e1);

            egui::Grid::new("a_to_b_3").num_columns(2).show(ui, |ui| {
                ui.label("k_0:");
                ui.label(hex::encode(k_0));
                ui.end_row();
                ui.label("k_1:");
                ui.label(hex::encode(k_1));
                ui.end_row();
                ui.label("e0:");
                ui.label(e0);
                ui.end_row();
                ui.label("e1:");
                ui.label(e1);
                ui.end_row();
            });
        });
        ui.collapsing("Oblivious Transfer Protocol (Bob)", |ui| {
            let k_c_p = self.a_point * self.b_scalar;
            let k_c = Sha256::digest(k_c_p.to_encoded_point(false).as_bytes())
                .as_slice()
                .try_into()
                .unwrap();
            let e_c = if self.c == C::C0 { &self.e0 } else { &self.e1 };
            let m_c = libaes::Cipher::new_256(&k_c).cbc_decrypt(&k_c, e_c);

            egui::Grid::new("b_1").num_columns(2).show(ui, |ui| {
                ui.label("k_c:");
                ui.label(hex::encode(k_c));
                ui.end_row();
                ui.label("e_c:");
                ui.label(hex::encode(e_c));
                ui.end_row();
                ui.label("m_c:");
                ui.label(String::from_utf8(m_c).unwrap());
                ui.end_row();
            });
        });
    }
}

impl Default for DemoPane {
    fn default() -> Self {
        let a = p256::Scalar::random(thread_rng());
        let ahex = format!("{:x}", a.to_bytes());
        let b = p256::Scalar::random(thread_rng());
        let bhex = format!("{:x}", b.to_bytes());

        Self {
            m0: String::new(),
            m1: String::new(),
            a: ahex,
            b: bhex,
            c: C::C0,
            a_scalar: a,
            b_scalar: b,
            a_point: ProjectivePoint::IDENTITY,
            b_point: ProjectivePoint::IDENTITY,
            e0: Vec::new(),
            e1: Vec::new(),
        }
    }
}

fn text_field(text: &mut dyn TextBuffer) -> TextEdit {
    TextEdit::singleline(text)
        .font(egui::FontSelection::FontId(FontId::new(
            12.,
            egui::FontFamily::Monospace,
        )))
        .desired_width(f32::INFINITY)
}
