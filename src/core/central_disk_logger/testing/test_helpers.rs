#[derive(Clone, Debug, PartialEq)]
pub struct MockTaskStruct {
    pub larger_than_zero: i32,
}
#[derive(Debug, PartialEq)]
pub struct MockConversionError;

#[derive(Clone, PartialEq, prost::Message)]
pub struct MockTaskProto {
    #[prost(int32, tag = "1")]
    pub larger_than_zero: i32,
}

impl TryFrom<MockTaskStruct> for MockTaskProto {
    type Error = MockConversionError;

    fn try_from(task_struct: MockTaskStruct) -> Result<Self, Self::Error> {
        if task_struct.larger_than_zero <= 0 {
            Err(MockConversionError)
        } else {
            Ok(MockTaskProto {
                larger_than_zero: task_struct.larger_than_zero,
            })
        }
    }
}
