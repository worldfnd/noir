use noirc_evaluator::ssa::{
    interpreter::tests::{
        executes_with_no_errors, expect_value, expect_values, expect_values_with_args,
        from_constant, expect_printed_output, from_u32_slice,
    },
    ir::types::NumericType,
};

fn main_empty() {
    let ssa = r#"
acir(inline) fn test_sha256_compression f0 {
  b0():
    v2 = make_array [u32 2147483648, u32 0, u32 0, u32 0, u32 0, u32 0, u32 0, u32 0, u32 0, u32 0, u32 0, u32 0, u32 0, u32 0, u32 0, u32 0] : [u32; 16]	// test_programs/execution_success/a_1_mul/src/main.nr:5:72
    v11 = make_array [u32 1779033703, u32 3144134277, u32 1013904242, u32 2773480762, u32 1359893119, u32 2600822924, u32 528734635, u32 1541459225] : [u32; 8]	// test_programs/execution_success/a_1_mul/src/main.nr:6:101
    v13 = call sha256_compression(v2, v11) -> [u32; 8]	// test_programs/execution_success/a_1_mul/src/main.nr:8:18
    return v13
}
    "#;
    let values = expect_value(ssa);
    println!("{:?}", values);
    // println!("{:?}", values);
}

fn check_expiry(){
  let ssa = r#"
g0 = u32 38
g1 = u32 65

acir(inline) fn main f0 {
  b0(v2: [u8; 95], v3: [u8; 8]):
    v5, v6, v7, v8, v9, v10 = call f1(v2, v3) -> (u8, u8, u32, u8, u8, u32)	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/complete_age_check/src/main.nr:65:36
    return v5, v6, v7, v8, v9, v10
}
acir(inline) fn check_expiry f1 {
  b0(v2: [u8; 95], v3: [u8; 8]):
    v5 = call f2(v2) -> [u8; 90]                  	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:8:15
    v7, v8, v9 = call f4(v3) -> (u8, u8, u32)     	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:13:26
    v12, v13, v14 = call f3(v7, v8, v9, u32 20) -> (u8, u8, u32)	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:13:26
    v26 = make_array b"threshold_year"            	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:13:26
    call f5(v26)                                  	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:14:5
    call f6(v12, v13, v14)                        	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:15:5
    v30 = call f7(v12, v13, v14) -> [u8; 8]       	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:17:32
    v32 = make_array b"threshold_year_bytes"      	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:17:32
    call f8(v32)                                  	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:18:5
    call f9(v30)                                  	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:19:5
    v35, v36, v37 = call f4(v3) -> (u8, u8, u32)  	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:22:24
    v41 = make_array b"current_date"              	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:22:24
    call f10(v41)                                 	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:23:5
    call f6(v35, v36, v37)                        	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:24:5
    v44 = make_array [u8 0, u8 0, u8 0, u8 0, u8 0, u8 0] : [u8; 6]	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:26:34
    v45 = allocate -> &mut [u8; 6]                	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:26:34
    store v44 at v45                              	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:26:34
    v49 = make_array b"expiry_date_bytes"         	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:26:34
    call f11(v49)                                 	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:27:5
    v51 = load v45 -> [u8; 6]                     	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:27:5
    call f12(v51)                                 	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:28:5
    v54 = call f13(v2) -> u1                      	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:30:8
    jmpif v54 then: b1, else: b2
  b1():
    v59 = call f14(v5, u32 38, u32 44) -> [u8; 6] 	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:31:29
    store v59 at v45                              	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:31:29
    jmp b3()
  b2():
    v57 = call f14(v5, u32 65, u32 71) -> [u8; 6] 	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:37:29
    store v57 at v45                              	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:37:29
    jmp b3()
  b3():
    v60 = load v45 -> [u8; 6]                     	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:37:29
    v62 = array_get v30, index u32 2 -> u8        	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:46:10
    v64 = array_get v30, index u32 3 -> u8        	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:46:35
    v65 = make_array [v62, v64] : [u8; 2]         	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:46:35
    v67, v68, v69 = call f15(v60, v65) -> (u8, u8, u32)	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:44:23
    v70 = make_array b"expiry_date"               	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:44:23
    call f16(v70)                                 	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:48:5
    call f6(v67, v68, v69)                        	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/data-check/expiry/src/lib.nr:49:5
    return v35, v36, v37, v67, v68, v69
}
acir(inline) fn get_mrz_from_dg1 f2 {
  b0(v2: [u8; 95]):
    v5 = make_array [u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0] : [u8; 90]	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:351:20
    v6 = allocate -> &mut [u8; 90]                	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:351:20
    store v5 at v6                                	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:351:20
    jmp b1(u32 0)
  b1(v3: u32):
    v9 = lt v3, u32 90                            	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:352:17
    jmpif v9 then: b2, else: b3
  b2():
    v12 = add v3, u32 5                           	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:353:22
    v14 = lt v12, u32 95                          	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:353:22
    constrain v14 == u1 1, "Index out of bounds"  	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:353:22
    v16 = array_get v2, index v12 -> u8           	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:353:18
    v17 = load v6 -> [u8; 90]                     	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:353:18
    v18 = lt v3, u32 90                           	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:353:18
    constrain v18 == u1 1, "Index out of bounds"  	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:353:18
    v19 = array_set v17, index v3, value v16      	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:353:9
    v21 = unchecked_add v3, u32 1                 	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:353:9
    store v19 at v6                               	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:353:9
    v22 = unchecked_add v3, u32 1                 	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:353:9
    jmp b1(v22)
  b3():
    v10 = load v6 -> [u8; 90]                     	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:353:9
    return v10
}
acir(inline) fn add_years f3 {
  b0(v2: u8, v3: u8, v4: u32, v5: u32):
    v6 = add v4, v5                               	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:253:56
    return v2, v3, v6
}
acir(inline) fn from_bytes_long_year f4 {
  b0(v2: [u8; 8]):
    v4 = array_get v2, index u32 0 -> u8          	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:65:56
    v6 = call f18(v4) -> u8                       	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:65:30
    v8 = array_get v2, index u32 1 -> u8          	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:66:57
    v9 = call f18(v8) -> u8                       	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:66:31
    v11 = array_get v2, index u32 2 -> u8         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:67:56
    v12 = call f18(v11) -> u8                     	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:67:30
    v14 = array_get v2, index u32 3 -> u8         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:68:57
    v15 = call f18(v14) -> u8                     	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:68:31
    v16 = cast v6 as u32                          	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:70:25
    v18 = mul v16, u32 1000                       	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:70:25
    v19 = cast v9 as u32                          	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:71:15
    v21 = mul v19, u32 100                        	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:71:15
    v22 = add v18, v21                            	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:70:25
    v23 = cast v12 as u32                         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:72:15
    v25 = mul v23, u32 10                         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:72:15
    v26 = add v22, v25                            	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:70:25
    v27 = cast v15 as u32                         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:73:15
    v28 = add v26, v27                            	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:70:25
    v30 = array_get v2, index u32 4 -> u8         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:75:57
    v31 = call f18(v30) -> u8                     	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:75:31
    v33 = array_get v2, index u32 5 -> u8         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:76:58
    v34 = call f18(v33) -> u8                     	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:76:32
    v36 = mul v31, u8 10                          	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:78:21
    v37 = add v36, v34                            	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:78:21
    v39 = array_get v2, index u32 6 -> u8         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:80:55
    v40 = call f18(v39) -> u8                     	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:80:29
    v42 = array_get v2, index u32 7 -> u8         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:81:56
    v43 = call f18(v42) -> u8                     	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:81:30
    v44 = mul v40, u8 10                          	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:83:19
    v45 = add v44, v43                            	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:83:19
    return v45, v37, v28
}
acir(inline) fn println f5 {
  b0(v2: [u8; 14]):
    call f26(u1 1, v2)                            	// std/lib.nr:40:9
    return
}
acir(inline) fn println f6 {
  b0(v2: u8, v3: u8, v4: u32):
    call f25(u1 1, v2, v3, v4)                    	// std/lib.nr:40:9
    return
}
acir(inline) fn to_bytes f7 {
  b0(v2: u8, v3: u8, v4: u32):
    v6 = make_array [u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0, u8 0] : [u8; 8]	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:153:34
    v7 = allocate -> &mut [u8; 8]                 	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:153:34
    store v6 at v7                                	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:153:34
    v9 = div v4, u32 1000                         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:155:30
    v10 = mul v9, u32 1000                        	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:156:44
    v11 = sub v4, v10                             	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:156:32
    v13 = div v11, u32 100                        	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:156:31
    v14 = mul v9, u32 1000                        	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:157:43
    v15 = sub v4, v14                             	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:157:31
    v16 = mul v13, u32 100                        	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:157:67
    v17 = sub v15, v16                            	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:157:31
    v19 = div v17, u32 10                         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:157:30
    v20 = mul v9, u32 1000                        	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:159:25
    v21 = sub v4, v20                             	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:159:13
    v22 = mul v13, u32 100                        	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:159:49
    v23 = sub v21, v22                            	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:159:13
    v24 = mul v19, u32 10                         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:159:73
    v25 = sub v23, v24                            	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:159:13
    v26 = truncate v9 to 8 bits, max_bit_size: 32 	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:161:39
    v27 = cast v26 as u8                          	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:161:39
    v29 = call f24(v27) -> u8                     	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:161:19
    v30 = load v7 -> [u8; 8]                      	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:161:19
    v32 = array_set v30, index u32 0, value v29   	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:161:9
    store v32 at v7                               	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:161:9
    v33 = truncate v13 to 8 bits, max_bit_size: 32	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:162:39
    v34 = cast v33 as u8                          	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:162:39
    v35 = call f24(v34) -> u8                     	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:162:19
    v36 = load v7 -> [u8; 8]                      	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:162:19
    v38 = array_set v36, index u32 1, value v35   	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:162:9
    store v38 at v7                               	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:162:9
    v39 = truncate v19 to 8 bits, max_bit_size: 32	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:163:39
    v40 = cast v39 as u8                          	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:163:39
    v41 = call f24(v40) -> u8                     	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:163:19
    v42 = load v7 -> [u8; 8]                      	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:163:19
    v44 = array_set v42, index u32 2, value v41   	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:163:9
    store v44 at v7                               	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:163:9
    v45 = truncate v25 to 8 bits, max_bit_size: 32	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:164:39
    v46 = cast v45 as u8                          	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:164:39
    v47 = call f24(v46) -> u8                     	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:164:19
    v48 = load v7 -> [u8; 8]                      	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:164:19
    v50 = array_set v48, index u32 3, value v47   	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:164:9
    store v50 at v7                               	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:164:9
    v52 = div v3, u8 10                           	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:166:31
    v53 = mul v52, u8 10                          	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:167:45
    v54 = sub v3, v53                             	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:167:32
    v55 = call f24(v52) -> u8                     	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:169:19
    v56 = load v7 -> [u8; 8]                      	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:169:19
    v58 = array_set v56, index u32 4, value v55   	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:169:9
    store v58 at v7                               	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:169:9
    v59 = call f24(v54) -> u8                     	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:170:19
    v60 = load v7 -> [u8; 8]                      	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:170:19
    v62 = array_set v60, index u32 5, value v59   	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:170:9
    store v62 at v7                               	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:170:9
    v63 = div v2, u8 10                           	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:172:29
    v64 = mul v63, u8 10                          	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:173:41
    v65 = sub v2, v64                             	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:173:30
    v66 = call f24(v63) -> u8                     	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:175:19
    v67 = load v7 -> [u8; 8]                      	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:175:19
    v69 = array_set v67, index u32 6, value v66   	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:175:9
    store v69 at v7                               	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:175:9
    v70 = call f24(v65) -> u8                     	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:176:19
    v71 = load v7 -> [u8; 8]                      	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:176:19
    v73 = array_set v71, index u32 7, value v70   	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:176:9
    store v73 at v7                               	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:176:9
    v74 = load v7 -> [u8; 8]                      	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:176:9
    return v74
}
acir(inline) fn println f8 {
  b0(v2: [u8; 20]):
    call f23(u1 1, v2)                            	// std/lib.nr:40:9
    return
}
acir(inline) fn println f9 {
  b0(v2: [u8; 8]):
    call f22(u1 1, v2)                            	// std/lib.nr:40:9
    return
}
acir(inline) fn println f10 {
  b0(v2: [u8; 12]):
    call f21(u1 1, v2)                            	// std/lib.nr:40:9
    return
}
acir(inline) fn println f11 {
  b0(v2: [u8; 17]):
    call f20(u1 1, v2)                            	// std/lib.nr:40:9
    return
}
acir(inline) fn println f12 {
  b0(v2: [u8; 6]):
    call f19(u1 1, v2)                            	// std/lib.nr:40:9
    return
}
acir(inline) fn is_id_card f13 {
  b0(v2: [u8; 95]):
    v4 = array_get v2, index u32 93 -> u8         	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:211:6
    v6 = eq v4, u8 0                              	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:211:6
    v7 = not v6                                   	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:211:6
    v9 = array_get v2, index u32 94 -> u8         	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:211:23
    v10 = eq v9, u8 0                             	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:211:23
    v11 = not v10                                 	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:211:23
    v12 = unchecked_mul v7, v11                   	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:211:5
    return v12
}
acir(inline) fn get_array_slice f14 {
  b0(v2: [u8; 90], v3: u32, v4: u32):
    v7 = make_array [u8 0, u8 0, u8 0, u8 0, u8 0, u8 0] : [u8; 6]	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:156:22
    v8 = allocate -> &mut [u8; 6]                 	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:156:22
    store v7 at v8                                	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:156:22
    jmp b1(v3)
  b1(v5: u32):
    v9 = lt v5, v4                                	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:157:21
    jmpif v9 then: b2, else: b3
  b2():
    v11 = sub v5, v3                              	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:158:15
    v13 = lt v5, u32 90                           	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:158:15
    constrain v13 == u1 1, "Index out of bounds"  	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:158:15
    v15 = array_get v2, index v5 -> u8            	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:158:28
    v16 = load v8 -> [u8; 6]                      	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:158:28
    v18 = lt v11, u32 6                           	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:158:28
    constrain v18 == u1 1, "Index out of bounds"  	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:158:28
    v19 = array_set v16, index v11, value v15     	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:158:9
    v21 = unchecked_add v11, u32 1                	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:158:9
    store v19 at v8                               	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:158:9
    v22 = unchecked_add v5, u32 1                 	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:158:9
    jmp b1(v22)
  b3():
    v10 = load v8 -> [u8; 6]                      	// /Users/ashishkumarsingh/Desktop/zk/ProveKit/noir-examples/noir-passport-examples/zkpassport_libs/utils/src/lib.nr:158:9
    return v10
}
acir(inline) fn from_bytes_short_year f15 {
  b0(v2: [u8; 6], v3: [u8; 2]):
    v5 = array_get v2, index u32 0 -> u8          	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:29:56
    v7 = call f18(v5) -> u8                       	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:29:30
    v9 = array_get v2, index u32 1 -> u8          	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:30:57
    v10 = call f18(v9) -> u8                      	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:30:31
    v11 = cast v7 as u32                          	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:32:29
    v13 = mul v11, u32 10                         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:32:29
    v14 = cast v10 as u32                         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:32:58
    v15 = add v13, v14                            	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:32:29
    v16 = allocate -> &mut u32                    	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:32:29
    store v15 at v16                              	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:32:29
    v18 = array_get v2, index u32 2 -> u8         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:34:57
    v19 = call f18(v18) -> u8                     	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:34:31
    v21 = array_get v2, index u32 3 -> u8         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:35:58
    v22 = call f18(v21) -> u8                     	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:35:32
    v24 = mul v19, u8 10                          	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:37:21
    v25 = add v24, v22                            	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:37:21
    v27 = array_get v2, index u32 4 -> u8         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:39:55
    v28 = call f18(v27) -> u8                     	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:39:29
    v30 = array_get v2, index u32 5 -> u8         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:40:56
    v31 = call f18(v30) -> u8                     	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:40:30
    v32 = mul v28, u8 10                          	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:42:19
    v33 = add v32, v31                            	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:42:19
    v34 = array_get v3, index u32 0 -> u8         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:44:63
    v35 = call f18(v34) -> u8                     	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:44:37
    v36 = array_get v3, index u32 1 -> u8         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:45:64
    v37 = call f18(v36) -> u8                     	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:45:38
    v38 = cast v35 as u32                         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:48:13
    v39 = mul v38, u32 10                         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:48:13
    v40 = cast v37 as u32                         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:48:49
    v41 = add v39, v40                            	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:48:13
    v42 = allocate -> &mut u32                    	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:48:13
    store v41 at v42                              	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:48:13
    v43 = load v16 -> u32                         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:48:13
    v44 = load v42 -> u32                         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:48:13
    v45 = lt v44, v43                             	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:55:12
    v46 = not v45                                 	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:55:12
    jmpif v46 then: b1, else: b2
  b1():
    v50 = load v16 -> u32                         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:55:12
    v52 = add v50, u32 2000                       	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:56:13
    store v52 at v16                              	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:56:13
    jmp b3()
  b2():
    v47 = load v16 -> u32                         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:56:13
    v49 = add v47, u32 1900                       	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:58:13
    store v49 at v16                              	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:58:13
    jmp b3()
  b3():
    v53 = load v16 -> u32                         	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:58:13
    return v33, v25, v53
}
acir(inline) fn println f16 {
  b0(v2: [u8; 11]):
    call f17(u1 1, v2)                            	// std/lib.nr:40:9
    return
}
brillig(inline) fn print_unconstrained f17 {
  b0(v2: u1, v3: [u8; 11]):
    v21 = make_array b"{\"kind\":\"string\",\"length\":11}"	// std/lib.nr:40:9
    call print(v2, v3, v21, u1 0)                 	// std/lib.nr:34:5
    return
}
acir(inline) fn get_number_from_utf8_code f18 {
  b0(v2: u8):
    v4 = lt v2, u8 48                             	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:10:12
    v5 = not v4                                   	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:10:12
    v7 = lt u8 57, v2                             	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:10:25
    v8 = not v7                                   	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:10:25
    v9 = unchecked_mul v5, v8                     	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:10:12
    constrain v4 == u1 0                          	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:10:12
    constrain v7 == u1 0                          	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:10:12
    v11 = sub v2, u8 48                           	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:11:5
    return v11
}
brillig(inline) fn print_unconstrained f19 {
  b0(v2: u1, v3: [u8; 6]):
    v27 = make_array b"{\"kind\":\"array\",\"length\":6,\"type\":{\"kind\":\"unsignedinteger\",\"width\":8}}"	// std/lib.nr:40:9
    call print(v2, v3, v27, u1 0)                 	// std/lib.nr:34:5
    return
}
brillig(inline) fn print_unconstrained f20 {
  b0(v2: u1, v3: [u8; 17]):
    v22 = make_array b"{\"kind\":\"string\",\"length\":17}"	// std/lib.nr:40:9
    call print(v2, v3, v22, u1 0)                 	// std/lib.nr:34:5
    return
}
brillig(inline) fn print_unconstrained f21 {
  b0(v2: u1, v3: [u8; 12]):
    v22 = make_array b"{\"kind\":\"string\",\"length\":12}"	// std/lib.nr:40:9
    call print(v2, v3, v22, u1 0)                 	// std/lib.nr:34:5
    return
}
brillig(inline) fn print_unconstrained f22 {
  b0(v2: u1, v3: [u8; 8]):
    v26 = make_array b"{\"kind\":\"array\",\"length\":8,\"type\":{\"kind\":\"unsignedinteger\",\"width\":8}}"	// std/lib.nr:40:9
    call print(v2, v3, v26, u1 0)                 	// std/lib.nr:34:5
    return
}
brillig(inline) fn print_unconstrained f23 {
  b0(v2: u1, v3: [u8; 20]):
    v22 = make_array b"{\"kind\":\"string\",\"length\":20}"	// std/lib.nr:40:9
    call print(v2, v3, v22, u1 0)                 	// std/lib.nr:34:5
    return
}
acir(inline) fn number_to_utf8_code f24 {
  b0(v2: u8):
    v4 = lt u8 9, v2                              	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:15:26
    v5 = not v4                                   	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:15:26
    v7 = unchecked_mul u1 1, v5                   	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:15:12
    constrain v4 == u1 0                          	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:15:12
    v10 = add v2, u8 48                           	// /Users/ashishkumarsingh/nargo/github.com/madztheo/noir-date.git/v0.4.4/src/lib.nr:16:5
    return v10
}
brillig(inline) fn print_unconstrained f25 {
  b0(v2: u1, v3: u8, v4: u8, v5: u32):
    v36 = make_array b"{\"kind\":\"struct\",\"name\":\"Date\",\"fields\":[[\"day\",{\"kind\":\"unsignedinteger\",\"width\":8}],[\"month\",{\"kind\":\"unsignedinteger\",\"width\":8}],[\"year\",{\"kind\":\"unsignedinteger\",\"width\":32}]]}"	// std/lib.nr:40:9
    call print(v2, v3, v4, v5, v36, u1 0)         	// std/lib.nr:34:5
    return
}
brillig(inline) fn print_unconstrained f26 {
  b0(v2: u1, v3: [u8; 14]):
    v22 = make_array b"{\"kind\":\"string\",\"length\":14}"	// std/lib.nr:40:9
    call print(v2, v3, v22, u1 0)                 	// std/lib.nr:34:5
    return
}
  "#;
  let arr: [u32; 95] = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 49, 49, 48, 54, 48, 57, 1, 1, 51, 48, 48, 56, 50, 53, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0];
  let dg1 = from_u32_slice(&arr, NumericType::Unsigned { bit_size: 8 });
  let current_date: [u32; 8] = [50, 48, 50, 52, 48, 54, 48, 55];
  let current_date_bytes = from_u32_slice(&current_date, NumericType::Unsigned { bit_size: 8 });

  let values = expect_values_with_args(ssa, vec![
        dg1,
        current_date_bytes
    ],
  );

  println!("{:?}", values);
}

fn main() {
    check_expiry();
    // main_empty();
}


