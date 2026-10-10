//! VehicleSeat attachment enums and model-local seated placement.
use glam::{Affine3A, Quat, Vec3};
use std::{collections::HashMap, path::Path};

/// SolarityClient native A2D3F0: DB2 attachment enums are not M2 lookup IDs.
pub fn seat_attachment_id(seat_enum: i32) -> Option<u32> {
    const IDS: [u32; 22] = [
        20, 34, 19, 21, 22, 17, 23, 24, 25, 15, 16, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 0,
    ];
    IDS.get(usize::try_from(seat_enum).ok()?).copied()
}

/// WoW model coordinates (z up) in the mount attachment's Godot coordinate frame.
pub fn seat_local_transform(offset: [f32; 3], rotation: [f32; 3]) -> Affine3A {
    let [x, y, z] = offset;
    let [yaw, pitch, roll] = rotation;
    let facing =
        Quat::from_rotation_y(yaw) * Quat::from_rotation_z(-pitch) * Quat::from_rotation_x(roll);
    Affine3A::from_rotation_translation(facing, Vec3::new(x, z, -y))
}

#[derive(Clone, Copy, Debug)]
pub struct VehicleSeat {
    pub attachment: u32,
    pub offset: [f32; 3],
    pub rotation: [f32; 3],
    pub animation: u16,
}

pub fn read_vehicle_seat(data_root: &Path, id: u32) -> Result<VehicleSeat, String> {
    let path = data_root.join("db2/12.1.0.69933/VehicleSeat.csv");
    let text =
        std::fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut lines = text.lines();
    let header = crate::csv_util::parse_csv_line(lines.next().ok_or("VehicleSeat CSV is empty")?);
    let id_column = crate::csv_util::header_index(&header, "ID", &path)?;
    let values = lines
        .map(crate::csv_util::parse_csv_line)
        .find(|row| {
            row.get(id_column)
                .is_some_and(|value| value == &id.to_string())
        })
        .ok_or_else(|| format!("VehicleSeat {id} absent from {}", path.display()))?;
    let row: HashMap<_, _> = header.into_iter().zip(values).collect();
    let attachment = seat_attachment_id(value(&row, "AttachmentID")?)
        .ok_or_else(|| format!("VehicleSeat {id}: invalid attachment enum"))?;
    let animation = value::<i16>(&row, "RideAnimLoop")?;
    Ok(VehicleSeat {
        attachment,
        offset: [
            value(&row, "AttachmentOffset_0")?,
            value(&row, "AttachmentOffset_1")?,
            value(&row, "AttachmentOffset_2")?,
        ],
        rotation: [
            value(&row, "PassengerYaw")?,
            value(&row, "PassengerPitch")?,
            value(&row, "PassengerRoll")?,
        ],
        animation: u16::try_from(animation)
            .map_err(|_| format!("VehicleSeat {id}: no passenger loop animation"))?,
    })
}

fn value<T: std::str::FromStr>(row: &HashMap<String, String>, column: &str) -> Result<T, String> {
    row.get(column)
        .ok_or_else(|| format!("Missing VehicleSeat {column}"))?
        .parse()
        .map_err(|_| format!("Invalid VehicleSeat {column}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Affine3A, Quat, Vec3};

    #[test]
    fn vehicle_seat_transform_uses_passenger_attachment_and_rotated_offset() {
        assert_eq!(seat_attachment_id(13), Some(39));
        assert_eq!(seat_attachment_id(14), Some(40));
        assert_eq!(seat_attachment_id(-1), None);
        assert_eq!(seat_attachment_id(22), None);
        let attachment = Affine3A::from_rotation_translation(
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            Vec3::new(10.0, 3.0, 20.0),
        );
        let local = seat_local_transform([2.0, 0.0, 1.0], [0.0, 0.0, 0.0]);
        let world = attachment * local;
        assert!(Vec3::from(world.translation).abs_diff_eq(Vec3::new(10.0, 4.0, 18.0), 1e-5));
    }
}
